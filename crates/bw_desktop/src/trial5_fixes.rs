//! Tests for the fixes from the fifth two-agent trial: the interface answers
//! for what the blind seats could not work out.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game, Mode, Screen};
    use bw_core::{Faction, Kind, Pos};
    use bw_sim::{Command, Event, EventKind};
    use std::path::PathBuf;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn settle(game: &mut Game, ticks: usize) {
        for _ in 0..ticks {
            game.tick();
        }
    }

    #[test]
    fn d_deploys_the_packed_ones_and_packs_only_when_all_are_deployed() {
        let mut g = game();
        let first = g.world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        let second = g.world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(32, 60));
        g.issue(Command::Deploy { units: vec![first] });
        settle(&mut g, 40);
        assert!(g.world.entities.iter().any(|e| e.id == first && e.deployed));
        g.selected = vec![first, second];
        assert!(!g.deploy_packs());
        g.action(Action::Deploy);
        settle(&mut g, 40);
        let both_deployed = g
            .world
            .entities
            .iter()
            .filter(|e| (e.id == first || e.id == second) && e.deployed)
            .count();
        assert_eq!(both_deployed, 2, "the deployed one kept its state");
        assert!(g.deploy_packs(), "with all deployed, the card offers PACK");
        // D never packs since rules 14: a second press leaves them deployed.
        g.action(Action::Deploy);
        settle(&mut g, 40);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| (e.id == first || e.id == second) && e.deployed)
                .count(),
            2
        );
    }

    #[test]
    fn a_move_to_a_deployed_group_packs_it_and_says_so() {
        let mut g = game();
        let bulwark = g.world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        g.issue(Command::Deploy {
            units: vec![bulwark],
        });
        settle(&mut g, 40);
        g.selected = vec![bulwark];
        g.issue(Command::Move {
            units: vec![bulwark],
            target: Pos::cell(40, 60),
            queued: false,
        });
        assert!(g.message.contains("PACKED"), "{}", g.message);
        settle(&mut g, 45);
        let e = g.world.entities.iter().find(|e| e.id == bulwark).unwrap();
        assert!(!e.deployed, "the move packed it");
        assert!(matches!(e.order, bw_sim::Order::Move { .. }));
    }

    #[test]
    fn two_sites_from_one_selection_get_two_builders() {
        let mut g = game();
        let workers: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .take(2)
            .collect();
        assert_eq!(workers.len(), 2);
        g.selected = workers.clone();
        let first = g.builder_for(Pos::cell(20, 60)).unwrap();
        g.recent_builders.push((first, g.world.tick));
        let second = g.builder_for(Pos::cell(20, 62)).unwrap();
        assert_ne!(first, second, "the second site goes to the other worker");
    }

    #[test]
    fn space_during_a_placement_keeps_the_worker_selected() {
        let mut g = game();
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        g.selected = vec![worker];
        g.mode = Mode::Build(Kind::Works);
        g.key("Space", false, false);
        assert_eq!(g.selected, vec![worker]);
        g.mode = Mode::Context;
        g.key("Space", false, false);
        assert!(
            g.selected.iter().all(|id| *id != worker),
            "Space selects the headquarters otherwise"
        );
    }

    #[test]
    fn a_box_may_start_on_the_header() {
        let mut g = game();
        g.render();
        // A spot on the header between its controls.
        let x = (0..g.canvas.width() as i32)
            .rev()
            .find(|&x| !g.buttons.iter().any(|b| b.contains(x, 4)) && x < 900)
            .expect("bare header");
        g.left_down(x, 4);
        assert!(g.drag.is_some(), "a drag from the header begins a box");
        let (_, y) = g.drag.unwrap();
        assert!(y >= g.world_view().top);
        g.left_up(x + 60, 300, false);
        assert!(g.drag.is_none());
    }

    #[test]
    fn idle_with_no_idle_worker_selects_the_nearest_worker() {
        let mut g = game();
        settle(&mut g, 5);
        assert!(
            g.idle_worker_ids().is_empty(),
            "starting workers gather on their own"
        );
        g.key("I", false, false);
        assert_eq!(g.selected.len(), 1);
        assert!(g.message.contains("nearest worker"), "{}", g.message);
        let id = g.selected[0];
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.id == id && e.kind.is_worker())
        );
    }

    #[test]
    fn a_lane_switch_raises_a_card_and_a_loss_at_home_says_at_base() {
        let mut g = game();
        settle(&mut g, 2);
        let before = std::collections::BTreeMap::new();
        g.world.events.push(Event {
            tick: g.world.tick,
            kind: EventKind::GateChanged,
            player: None,
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        });
        g.ux.alerts.observe(&g.world, &before);
        assert_eq!(
            g.ux.alerts.entries[0].kind,
            crate::field_alerts::AlertKind::LanesSwitched
        );
        assert_eq!(g.ux.alerts.entries[0].label(), "LANES SWITCHED");
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        g.ux.alerts.push(
            crate::field_alerts::AlertKind::MachinesLost,
            hq,
            g.world.tick,
        );
        assert_eq!(g.ux.alerts.entries[0].label(), "MACHINE LOST AT BASE");
    }

    #[test]
    fn the_stats_row_drops_segments_instead_of_clipping() {
        let full = "DMG 3 / REACH 1 / SPEED 4 / SIGHT 8";
        assert_eq!(
            crate::native_ui::fit_segments(full, 300, 2),
            "DMG 3 / REACH 1 / SPEED 4"
        );
        assert_eq!(crate::native_ui::fit_segments(full, 600, 2), full);
        assert_eq!(crate::native_ui::fit_segments(full, 120, 2), "DMG 3");
    }

    #[test]
    fn the_interface_scale_follows_the_display_and_the_setting() {
        let mut g = game();
        assert_eq!(g.ui_scale(), 2);
        g.display_scale = 2;
        assert_eq!(g.ui_scale(), 2, "a 1280x720 window holds no more");
        g.resize_view(1920, 1080);
        assert_eq!(
            g.ui_scale(),
            3,
            "a two-to-one display at 1920x1080 fits three"
        );
        g.resize_view(2560, 1440);
        assert_eq!(g.ui_scale(), 4);
        g.ux.preferences.interface_scale = 2;
        assert_eq!(g.ui_scale(), 2, "the setting fixes it");
        g.display_scale = 1;
        g.ux.preferences.interface_scale = 0;
        g.resize_view(1920, 1080);
        assert_eq!(g.ui_scale(), 2, "a plain display keeps the old size");
    }

    #[test]
    fn the_result_screen_does_not_show_the_guide_behind_it() {
        let mut g = game();
        g.open_screen(Screen::Help);
        g.render();
        // The Guide's header runs along the top of the menu canvas, above
        // where the result panel stands.
        let tab = g.ui_canvas.pixels[(30 * 4)..(30 * 4 + 4)].to_vec();
        assert_ne!(tab, crate::canvas::INK.to_vec(), "the tab bar is drawn");
        g.screen = Screen::Match;
        g.world.outcome = Some(bw_sim::Outcome::Victory(0));
        g.render();
        let after = g.ui_canvas.pixels[(30 * 4)..(30 * 4 + 4)].to_vec();
        assert_eq!(
            after,
            vec![0, 0, 0, 0],
            "the result stands over the field, not the Guide"
        );
    }

    #[test]
    fn the_roster_page_shows_one_faction_with_a_toggle() {
        let mut g = game();
        g.ux.help_page = crate::menus::ROSTER_PAGE;
        g.ux.roster_faction = Some(Faction::Assembly);
        g.open_screen(Screen::Help);
        g.render();
        assert!(g.buttons.iter().any(|b| b.label == "UNION"));
        assert!(g.buttons.iter().any(|b| b.label == "ASSEMBLY"));
        g.action(Action::RosterFaction(Faction::Union));
        assert_eq!(g.ux.roster_faction, Some(Faction::Union));
    }

    #[test]
    fn the_header_carries_a_salvage_rate_and_the_idle_button_a_worker_count() {
        let mut g = game();
        settle(&mut g, 2);
        g.deposits.push_back((g.world.tick, 40));
        let state = g.console_state();
        assert!(state.resources.salvage_rate_per_minute.unwrap_or(0) > 0);
        g.render();
        let idle = g
            .buttons
            .iter()
            .find(|b| b.label.starts_with("IDLE"))
            .unwrap();
        assert!(idle.label.contains('/'), "{}", idle.label);
        assert!(idle.enabled, "a worker can be found even with none idle");
    }

    #[test]
    fn full_pressure_is_noted_once() {
        let mut g = game();
        g.world.players[0].pressure = g.world.players[0].pressure_cap;
        settle(&mut g, 70);
        assert!(g.message.contains("trickles into salvage"), "{}", g.message);
        assert!(g.pressure_full_noted);
    }

    #[test]
    fn the_build_refusal_names_the_key_and_the_hold_message_the_cancel() {
        let mut g = game();
        g.selected.clear();
        g.action(Action::Build(Kind::Tower));
        assert!(g.message.starts_with("V builds a "), "{}", g.message);
        let bulwark = g.world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        g.selected = vec![bulwark];
        g.action(Action::Hold);
        assert!(g.message.contains("cancelled"), "{}", g.message);
    }

    #[test]
    fn a_deploy_order_says_whether_it_packs() {
        let mut g = game();
        let bulwark = g.world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        g.selected = vec![bulwark];
        g.action(Action::Deploy);
        assert_eq!(g.message, "Deploy order sent.");
        settle(&mut g, 40);
        g.action(Action::Pack);
        assert_eq!(g.message, "Pack order sent.");
    }
}
