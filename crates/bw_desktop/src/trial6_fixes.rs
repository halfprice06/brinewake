//! Tests for the fixes from the sixth two-agent trial: the hold row and the
//! DRY words on the sluice card, crossing plates with counts, the new
//! alert cards, death causes, the result's cause, RECLAIM and BOARD, the
//! placement plate, field controls that fire on release, and the harness
//! state a referee reads.
#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, WHITE};
    use crate::field_alerts::{AlertHistory, AlertKind};
    use crate::game::{Action, Game, Mode, enemy_edge_offsets};
    use crate::native_ui::{fit_small, wrap_small};
    use bw_core::{Kind, Pos};
    use bw_sim::{Command, Event, EventKind, Outcome};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        // These checks read the sluice card, so it is pinned open.
        game.ux.preferences.tide_card = true;
        game
    }

    fn settle(game: &mut Game, ticks: usize) {
        for _ in 0..ticks {
            game.tick();
        }
    }

    fn label_of(game: &Game, action: &Action) -> Option<String> {
        game.buttons
            .iter()
            .find(|b| b.action == *action)
            .map(|b| b.label.clone())
    }

    fn event(kind: EventKind) -> Event {
        Event {
            tick: 0,
            kind,
            player: Some(0),
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        }
    }

    #[test]
    fn small_fit_and_wrap_cut_by_six_pixel_glyphs() {
        assert_eq!(fit_small("ABCDEFGHIJ", 36, 1), "ABC...");
        assert_eq!(fit_small("ABCDEF", 36, 1), "ABCDEF");
        assert_eq!(
            wrap_small("16 MACHINES LOST AT BASE TO LOOMS 5S", 23, 2),
            vec!["16 MACHINES LOST AT", "BASE TO LOOMS 5S"]
        );
        assert_eq!(wrap_small("SHORT", 23, 2), vec!["SHORT"]);
    }

    #[test]
    fn the_card_reads_dry_and_carries_the_hold_row() {
        let mut g = game();
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 200;
        g.render();
        assert_eq!(
            label_of(&g, &Action::SetTide(bw_sim::Arm::NORTH)).as_deref(),
            Some("DRY N")
        );
        assert_eq!(
            label_of(&g, &Action::SetTide(bw_sim::Arm::SOUTH)).as_deref(),
            Some("DRY S")
        );
        let s = g.ui_scale();
        let route = g.route_bounds();
        assert_eq!(
            route.h,
            93 * s,
            "status, tide, hold row, checklist and buttons"
        );
        for b in g
            .buttons
            .iter()
            .filter(|b| matches!(b.action, Action::SetTide(_) | Action::Flood))
        {
            assert_eq!(b.y, route.y + 64 * s, "the buttons sit under the checklist");
        }
        g.action(Action::SetTide(bw_sim::Arm::SOUTH));
        assert!(g.message.contains("south dries"), "{}", g.message);
        assert!(g.hold_gauge().is_none());
        for mouths in bw_sim::CROSSING_MOUTHS {
            let (x, y) = mouths[0].cell_xy();
            g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(x, y));
        }
        settle(&mut g, 2);
        assert!(g.world.lane_hold[0] > 0);
        assert_eq!(g.hold_gauge(), Some((0, g.world.lane_hold[0])));
        assert!(crate::qol::lane_presence(&g.world, 0).0);
        g.world.gate.owner = None;
        g.render();
        assert_eq!(
            g.route_bounds().h,
            71 * s,
            "the hold row and checklist stay without the buttons"
        );
    }

    #[test]
    fn crossing_labels_carry_the_count_and_badges_stay_in_view() {
        use crate::tactics::crossing_label;
        assert_eq!(
            crossing_label(0, 0, true, false, false, [30, 0]),
            "NW HELD 89S"
        );
        assert_eq!(
            crossing_label(
                1,
                1,
                false,
                true,
                false,
                [0, bw_content::TIDE_HOLD_TICKS - 300]
            ),
            "SE ENEMY 10S"
        );
        assert_eq!(
            crossing_label(0, 1, true, true, true, [5, 5]),
            "NE CONTESTED"
        );
        assert_eq!(crossing_label(1, 0, false, false, false, [0, 0]), "SW");
        // At rest a mouth shows only its count, and none while contested.
        use crate::tactics::hold_count;
        assert_eq!(
            hold_count(true, false, false, [30, 0], bw_content::TIDE_HOLD_TICKS).as_deref(),
            Some("89")
        );
        assert_eq!(
            hold_count(true, true, true, [5, 5], bw_content::TIDE_HOLD_TICKS),
            None
        );
        assert_eq!(
            hold_count(false, false, false, [0, 0], bw_content::TIDE_HOLD_TICKS),
            None
        );
        let mut canvas = Canvas::new(200, 100);
        canvas.clear([0, 0, 0, 255]);
        crate::field_labels::draw_badge(&mut canvas, "WIDE", -50, 20, WHITE);
        assert_ne!(
            canvas.get(0, 18),
            Some([0, 0, 0, 255]),
            "a badge past the left edge slides inside the picture"
        );
        crate::field_labels::draw_badge(&mut canvas, "89", 100, -30, WHITE);
        assert_eq!(canvas.get(100, 0), Some([0, 0, 0, 255]), "no half badge");
    }

    #[test]
    fn enemy_hold_contested_and_wreck_cards_and_seen_merge() {
        let mut g = game();
        let mut history = AlertHistory::default();
        let before = BTreeMap::new();
        g.world.events.clear();
        g.world.lane_hold[1] = 1;
        history.observe(&g.world, &before);
        assert_eq!(history.entries[0].kind, AlertKind::EnemyHold);
        assert!(history.entries[0].kind.urgent());
        g.world.lane_hold[1] = 2;
        history.observe(&g.world, &before);
        assert_eq!(
            history
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::EnemyHold)
                .count(),
            1,
            "a rising gauge is one card"
        );
        // An enemy holder on a lane we hold: CONTESTED once.
        let (wx, wy) = bw_sim::CROSSING_MOUTHS[0][0].cell_xy();
        let (ex, ey) = bw_sim::CROSSING_MOUTHS[0][1].cell_xy();
        g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(wx, wy));
        g.world
            .spawn_for_tests(1, Kind::Reedguard, Pos::cell(ex, ey));
        history.observe(&g.world, &before);
        history.observe(&g.world, &before);
        assert_eq!(
            history
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::CrossingContested(0))
                .count(),
            1
        );
        assert_eq!(
            history
                .entries
                .iter()
                .find(|a| a.kind == AlertKind::CrossingContested(0))
                .map(|a| a.label()),
            Some("N LANE CONTESTED".into())
        );
        // A wreck ran dry east of home.
        let mut emptied = event(EventKind::WreckEmptied);
        emptied.from = Some(Pos::cell(40, 60));
        emptied.text = "workers idle".into();
        g.world.events = vec![emptied];
        history.observe(&g.world, &before);
        let wreck = history
            .entries
            .iter()
            .find(|a| matches!(a.kind, AlertKind::WreckEmpty(_)))
            .expect("a wreck card");
        assert_eq!(
            wreck.caption(5),
            "WRECK EMPTY AT N LANE, YOUR BANK WORKERS IDLE 5S AGO"
        );
        // Two sightings at one place are one card.
        let mut seen_a = event(EventKind::EnemySeen);
        seen_a.to = Some(Pos::cell(40, 60));
        seen_a.amount = 1;
        let mut seen_b = event(EventKind::EnemySeen);
        seen_b.to = Some(Pos::cell(43, 62));
        seen_b.amount = 2;
        g.world.events = vec![seen_a, seen_b];
        history.observe(&g.world, &before);
        assert_eq!(
            history
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::EnemySeen)
                .count(),
            1
        );
        // Two losses at home to Looms: one card with the cause.
        let home = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        let (hx, hy) = home.cell_xy();
        let mut before = BTreeMap::new();
        before.insert(9001, (Kind::Sounder, 0u8, Pos::cell(hx + 2, hy)));
        before.insert(9002, (Kind::Sounder, 0u8, Pos::cell(hx + 3, hy)));
        let mut deaths = Vec::new();
        for id in [9001, 9002] {
            let mut death = event(EventKind::Death);
            death.entity = Some(id);
            death.cause = Some(Kind::Loom);
            death.text = "SOUNDER".into();
            deaths.push(death);
        }
        g.world.events = deaths;
        history.observe(&g.world, &before);
        let lost = history
            .entries
            .iter()
            .find(|a| a.kind == AlertKind::MachinesLost)
            .expect("a loss card");
        assert_eq!(lost.caption(3), "2 MACHINES LOST AT BASE TO LOOMS 3S AGO");
    }

    #[test]
    fn death_rows_and_results_name_the_cause() {
        use crate::dock_log::{death_words, result_words};
        let mut lost = event(EventKind::Death);
        lost.text = "SOUNDER".into();
        lost.cause = Some(Kind::Loom);
        assert_eq!(death_words(&lost), "LOST SOUNDER TO LOOM");
        let mut killed = event(EventKind::Death);
        killed.player = Some(1);
        killed.text = "REEDGUARD".into();
        assert_eq!(death_words(&killed), "KILLED REEDGUARD");
        assert_eq!(
            result_words(Some(&Outcome::Victory(1)), false, true, false).1,
            "THE ENEMY HELD BOTH LANES FOR 90 SECONDS."
        );
        assert_eq!(
            result_words(Some(&Outcome::Victory(0)), false, true, false).1,
            "YOU HELD BOTH LANES FOR 90 SECONDS."
        );
        assert_eq!(
            result_words(Some(&Outcome::Victory(0)), false, false, false).1,
            "THE ENEMY HEADQUARTERS FELL."
        );
        let mut g = game();
        let mut victory = event(EventKind::Victory);
        victory.player = None;
        victory.text = "SILT ASSEMBLY HOLDS THE TIDE".into();
        g.world.events = vec![victory];
        g.ux.dock.after_authoritative_tick(&g.world);
        assert!(g.ux.dock.outcome_text.contains("HOLDS THE TIDE"));
    }

    #[test]
    fn reclaim_sits_on_the_hq_card_and_the_loom_note_fires_once() {
        let mut g = game();
        g.home();
        g.world.players[0].pressure = 150;
        g.render();
        assert_eq!(label_of(&g, &Action::Reclaim).as_deref(), Some("RECLAIM"));
        let hint = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Reclaim)
            .map(|b| b.hint.clone())
            .unwrap();
        assert!(hint.contains("60 SALVAGE"), "{hint}");
        assert_eq!(g.world.players[0].pressure, 150);
        g.key("R", false, false);
        assert!(
            matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(Command::Reclaim { .. })
            ),
            "{:?}",
            g.world.command_log.last()
        );
        assert!(g.message.starts_with("RECLAIM"), "{}", g.message);
        g.world.players[0].pressure = 0;
        assert!(g.action_reason(&Action::Reclaim).is_some());
        // The pending RECLAIM executes on the next tick: give it its price.
        g.world.players[0].pressure = 150;
        assert!(
            crate::ux::kind_tooltip(Kind::Riveter, "Q").contains("SURGE (Z)"),
            "the Loom counter sits in the Riveter's tooltip"
        );
        let home = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        let (hx, hy) = home.cell_xy();
        g.world
            .spawn_for_tests(1, Kind::Loom, Pos::cell(hx + 4, hy));
        settle(&mut g, 1);
        assert!(g.loom_noted);
        assert!(g.message.starts_with("Enemy Looms"), "{}", g.message);
    }

    #[test]
    fn the_ghost_plate_names_the_refusal_and_a_condenser_needs_only_its_well_seen() {
        use crate::ux::Refusal;
        let mut g = game();
        assert_eq!(
            g.placement_refusal(Kind::Works, g.world.map.gate_pos),
            Some(Refusal::SluiceApproach)
        );
        assert_eq!(Refusal::SluiceApproach.short(), "SLUICE APPROACH");
        let far = Pos::cell(100, 40);
        assert_eq!(
            g.placement_cell_refusal(far, true),
            Some(Refusal::Unexplored)
        );
        assert_ne!(
            g.placement_cell_refusal(far, false),
            Some(Refusal::Unexplored)
        );
        g.world.revealed = true;
        assert_eq!(
            g.placement_refusal(Kind::Condenser, Pos::cell(30, 60)),
            Some(Refusal::NoWell)
        );
        assert_eq!(Refusal::NoWell.short(), "NEEDS A WELL");
        assert_eq!(
            Refusal::Occupied.sentence(),
            "This space is occupied. Choose clear ground; Esc cancels."
        );
    }

    #[test]
    fn overlay_buttons_fire_on_release_and_yield_to_a_pending_mode() {
        // The sluice card's fold is the control that still stands on the
        // field; IDLE, ARMY and WORKS moved into the dock.  The fold is a
        // saved preference, so this game keeps its own data folder.
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("brinewake-fold-{}", std::process::id())),
        );
        g.start();
        g.world.ai_enabled = false;
        g.resize_view(1280, 720);
        g.selected.clear();
        g.ux.preferences.tide_card = true;
        g.render();
        let fold = g
            .buttons
            .iter()
            .find(|b| b.action == Action::CompactFieldCards)
            .cloned()
            .expect("the fold control");
        let folded = g.ux.preferences.compact_field_cards;
        let (cx, cy) = (fold.x + fold.w / 2, fold.y + fold.h / 2);
        g.left_down(cx, cy);
        assert_eq!(
            g.ux.preferences.compact_field_cards, folded,
            "nothing fires on the press"
        );
        assert!(g.pressed_overlay.is_some());
        g.left_up(cx, cy, false);
        assert_ne!(
            g.ux.preferences.compact_field_cards, folded,
            "the release folds the card"
        );
        assert!(g.pressed_overlay.is_none());
        g.render();
        let fold = g
            .buttons
            .iter()
            .find(|b| b.action == Action::CompactFieldCards)
            .cloned()
            .expect("the fold control");
        let (cx, cy) = (fold.x + fold.w / 2, fold.y + fold.h / 2);
        // A press that becomes a drag is a box over the ground.
        let folded = g.ux.preferences.compact_field_cards;
        g.left_down(cx, cy);
        g.left_up(cx - 60, cy + 80, false);
        assert!(g.pressed_overlay.is_none());
        assert!(g.drag.is_none());
        assert_eq!(g.ux.preferences.compact_field_cards, folded);
        // With a placement pending the control yields to the ground.
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .expect("a worker");
        g.selected = vec![worker];
        g.action(Action::Build(Kind::Works));
        assert_eq!(g.mode, Mode::Build(Kind::Works));
        g.left_down(cx, cy);
        assert!(g.pressed_overlay.is_none());
        assert_eq!(g.ux.preferences.compact_field_cards, folded);
    }

    #[test]
    fn space_only_looks_home_while_an_order_is_pending() {
        let mut g = game();
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(30, 60));
        for mode in [Mode::Attack, Mode::Gather, Mode::Face] {
            g.selected = vec![riveter];
            g.mode = mode;
            g.key("Space", false, false);
            assert_eq!(g.selected, vec![riveter], "{mode:?} keeps the selection");
        }
        g.mode = Mode::Context;
        g.key("Space", false, false);
        assert_ne!(
            g.selected,
            vec![riveter],
            "in no mode Space selects the headquarters"
        );
    }

    #[test]
    fn the_roster_page_clamps_when_the_kinds_fit_one_page() {
        let mut g = game();
        let kinds = [
            Kind::Riveter,
            Kind::Bulwark,
            Kind::Sounder,
            Kind::Tidewatch,
            Kind::Caulker,
            Kind::Caisson,
            Kind::Lifter,
            Kind::Hook,
        ];
        let mut ids = Vec::new();
        for (i, kind) in kinds.iter().enumerate() {
            ids.push(
                g.world
                    .spawn_for_tests(0, *kind, Pos::cell(30, 56 + i as i32)),
            );
        }
        g.selected = ids;
        g.ux.roster_page = 1;
        g.render();
        // The icon grid holds two rows of ten at 1280x720: eight kinds are
        // one page, and a stale page clamps back to it.
        assert_eq!(g.ux.roster_page, 0, "eight kinds fit one page: clamped");
        for kind in kinds {
            assert!(
                g.buttons
                    .iter()
                    .any(|b| b.action == Action::FilterSelection(kind, false)),
                "{kind:?} on the page"
            );
        }
        g.filter_selection(Kind::Hook, true);
        g.render();
        assert_eq!(g.ux.roster_page, 0);
    }

    #[test]
    fn the_card_offers_board_to_the_nearest_transport_with_room() {
        let mut g = game();
        let lifter = g.world.spawn_for_tests(0, Kind::Lifter, Pos::cell(30, 60));
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(28, 60));
        g.selected = vec![riveter];
        g.camera.center(Pos::cell(30, 60));
        g.render();
        let board = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Board)
            .cloned()
            .expect("the BOARD button");
        assert!(board.hint.contains("LIFTER"), "{}", board.hint);
        // E boards; B is BUILD only (trial 12).
        g.key("E", false, false);
        assert!(
            matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(Command::Board { transport, .. }) if *transport == lifter
            ),
            "{:?}",
            g.world.command_log.last()
        );
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == lifter) {
            e.cargo = vec![1, 2, 3, 4];
        }
        g.render();
        assert!(
            !g.buttons.iter().any(|b| b.action == Action::Board),
            "a full hold offers no BOARD"
        );
    }

    #[test]
    fn roster_roles_fit_two_lines_and_the_sluice_page_names_dry() {
        use crate::menus::{ROSTER_KINDS, guide_page, role_lines};
        for kind in ROSTER_KINDS {
            assert!(
                role_lines(kind).len() <= 2,
                "{kind:?}: {:?}",
                role_lines(kind)
            );
        }
        for kind in [Kind::Condenser, Kind::Dropoff, Kind::Tower] {
            assert!(ROSTER_KINDS.contains(&kind), "{kind:?} is on the roster");
        }
        assert!(
            guide_page(3, bw_core::Faction::Union)
                .0
                .iter()
                .any(|l| l.contains("DRY N / DRY S"))
        );
        assert!(
            guide_page(0, bw_core::Faction::Union)
                .1
                .contains(&("1-9", "RECALL, TWICE CENTRES"))
        );
    }

    #[test]
    fn crew_refusals_name_the_ceiling_or_the_buildings() {
        use crate::production_qol::crew_ceiling_reason;
        assert!(crew_ceiling_reason(90).starts_with("Crew is at the 90 ceiling"));
        assert!(crew_ceiling_reason(40).contains("a Works adds 12"));
        let g = game();
        assert!(
            g.rejection_reason("crew capacity exceeded")
                .contains("Works adds")
        );
    }

    #[test]
    fn the_enemy_edge_is_two_pixels_in_both_axes_at_scale_two() {
        assert!(enemy_edge_offsets(2).contains(&(0, 1)));
        assert_eq!(enemy_edge_offsets(1), &[(0, 0)]);
    }

    #[test]
    fn state_reports_tide_and_hold() {
        let g = game();
        let state = crate::agent::state_for_tests(&g);
        assert_eq!(state["tide"]["mode"], "Neutral");
        assert_eq!(state["hold"]["lanes"].as_array().map(|l| l.len()), Some(2));
        assert_eq!(state["hold"]["own_seconds_left"], 90);
    }
}
