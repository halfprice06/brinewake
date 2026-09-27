//! Tests for the eighth trial's HUD batch: the full-pressure warning names
//! RECLAIM, a site without a builder raises a card, the tide buttons say
//! what they open and that a deep lane is a wall, alert ages read "AGO",
//! the gather and research messages say what really happens, a queued unit
//! shows at once, the sluice card and field buttons fold, the command card
//! fits its words, the tooltips are smaller, and the worker's button says
//! it is the worker.
#[cfg(test)]
mod tests {
    use crate::field_alerts::AlertKind;
    use crate::game::{Action, Game};
    use bw_core::{Faction, Kind, Pos};
    use std::path::PathBuf;

    fn game_as(faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-t8hud-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn game() -> Game {
        game_as(Faction::Union)
    }

    /// Save the frame when `BW_TRIAL8_HUD_FRAMES` names a folder, for review.
    fn keep_frame(g: &Game, name: &str) {
        if let Ok(dir) = std::env::var("BW_TRIAL8_HUD_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&PathBuf::from(dir).join(format!("{name}.png")))
                .expect("frame");
        }
    }

    fn own(g: &Game, kind: Kind) -> u32 {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == kind)
            .map(|e| e.id)
            .expect("own entity")
    }

    fn settle(g: &mut Game, ticks: usize) {
        for _ in 0..ticks {
            g.tick();
        }
    }

    /// The command card's buttons as drawn: those in the dock's card.
    fn card_buttons(g: &Game) -> Vec<crate::game::Button> {
        let dock_top = g.world_view().bottom;
        let card_x = g.command_card_x();
        g.buttons
            .iter()
            .filter(|b| b.y >= dock_top && b.x >= card_x)
            .cloned()
            .collect()
    }

    /// Every button on the card shows its icon from the atlas (the words
    /// live in the tooltip), and every row of its tooltip fits whole:
    /// nothing cut to "WADE FASTE" or "HOLDS 4, F".
    fn assert_card_fits(g: &Game, what: &str) {
        let s = g.ui_scale();
        let buttons = card_buttons(g);
        assert!(!buttons.is_empty(), "{what}: a card");
        let atlas = g.atlas.as_ref().expect("the atlas");
        for b in buttons {
            let key = crate::lean_hud::icon_key(&b.action, g.faction)
                .unwrap_or_else(|| panic!("{what}: {:?} has no icon", b.action));
            let key = if b.action == Action::Formation {
                "ui_cmd_formation_compact".to_string()
            } else {
                key
            };
            assert!(
                atlas.sprites.contains_key(&key),
                "{what}: {key} is not in the atlas"
            );
            assert_eq!((b.w, b.h), (26 * s, 26 * s), "{what}: {:?}", b.action);
            let cap = ((crate::native_ui::TIP_W - 10) / 6) as usize;
            for row in b.hint.split('\n') {
                assert!(
                    row.chars().count() <= cap,
                    "{what}: {:?} tooltip row {row:?} is cut at {cap}",
                    b.label
                );
            }
            assert!(
                !b.hint.contains("S30P") && !b.hint.contains("P P"),
                "{what}: {:?} runs its price together: {:?}",
                b.label,
                b.hint
            );
        }
    }

    #[test]
    fn every_command_card_fits_its_words() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut g = game_as(faction);
            g.world.players[0].salvage = 5_000;
            g.world.players[0].pressure = 400;
            let hq = own(&g, Kind::Headquarters);
            g.selected = vec![hq];
            g.render();
            assert_card_fits(&g, "headquarters");
            keep_frame(&g, &format!("card-hq-{faction:?}"));
            for (i, kind) in [Kind::Works, Kind::Drydock, Kind::Dropoff, Kind::Condenser]
                .into_iter()
                .enumerate()
            {
                let id = g
                    .world
                    .spawn_for_tests(0, kind, Pos::cell(20 + 6 * i as i32, 40));
                g.selected = vec![id];
                g.render();
                assert_card_fits(&g, kind.name());
                keep_frame(&g, &format!("card-{}-{faction:?}", kind.name()));
            }
            let worker = own(&g, faction.worker());
            g.selected = vec![worker];
            g.render();
            assert_card_fits(&g, "worker");
            for kind in faction.army().into_iter().chain(faction.drydock_roles()) {
                let id = g.world.spawn_for_tests(0, kind, Pos::cell(30, 60));
                g.selected = vec![id];
                g.render();
                assert_card_fits(&g, kind.name());
            }
        }
    }

    #[test]
    fn prices_read_as_two_resources_and_keys_are_not_doubled() {
        let mut g = game();
        let drydock = g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(24, 40));
        g.selected = vec![drydock];
        g.render();
        let tracks = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::Upgrade(_)))
            .expect("the Tracks button");
        assert_eq!(tracks.hint, "P 150S+50P\nWADES FAST");
        let worker = own(&g, Kind::Hook);
        g.selected = vec![worker];
        g.render();
        let nest = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Build(Kind::Tower))
            .expect("the nest button");
        assert_eq!(nest.label, "NEST");
        assert!(nest.hint.starts_with("V 180S+30P"), "{}", nest.hint);
    }

    #[test]
    fn the_worker_button_says_it_is_the_worker() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut g = game_as(faction);
            g.selected = vec![own(&g, Kind::Headquarters)];
            g.render();
            let train = g
                .buttons
                .iter()
                .find(|b| b.action == Action::Train(faction.worker()))
                .expect("the worker button");
            assert_eq!(train.label, faction.worker().name());
            assert!(train.hint.ends_with("\nWORKER"), "{}", train.hint);
        }
    }

    #[test]
    fn a_queued_unit_shows_the_moment_it_is_ordered() {
        let mut g = game();
        g.world.players[0].salvage = 1_000;
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        g.action(Action::Train(Kind::Hook));
        let info = g.selected_production_info().expect("the panel");
        assert_eq!(info.heading, "HOOK / 1 QUEUED", "before the world has it");
        assert_eq!(info.status, "ORDER SENT / STARTING");
        settle(&mut g, 5);
        let info = g.selected_production_info().expect("the panel");
        assert_eq!(info.heading, "HOOK / 1 QUEUED", "once it has it");
        assert_ne!(info.status, "ORDER SENT / STARTING");
    }

    #[test]
    fn research_keeps_the_paused_queue_on_the_panel() {
        let mut g = game();
        g.world.players[0].salvage = 1_000;
        g.world.players[0].pressure = 150;
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        g.action(Action::Train(Kind::Hook));
        g.action(Action::Train(Kind::Hook));
        g.action(Action::Research(bw_content::Doctrine::Hauling));
        assert!(
            g.message.contains("HQ worker training pauses for 30s"),
            "{}",
            g.message
        );
        settle(&mut g, 5);
        let info = g.selected_production_info().expect("the panel");
        assert_eq!(info.heading, "HOOK / 2 QUEUED");
        assert!(info.status.contains("2 WAIT, PAUSED"), "{}", info.status);
        g.render();
        keep_frame(&g, "research-paused-queue");
    }

    #[test]
    fn alert_ages_say_ago() {
        let mut g = game();
        g.world.players[0].salvage = 1_000;
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        g.action(Action::Train(Kind::Hook));
        for _ in 0..1_500 {
            g.tick();
            if g.ux
                .alerts
                .entries
                .iter()
                .any(|a| matches!(a.kind, AlertKind::Ready(_)))
            {
                break;
            }
        }
        settle(&mut g, 90);
        let card =
            g.ux.alerts
                .entries
                .iter()
                .find(|a| matches!(a.kind, AlertKind::Ready(_)))
                .expect("a ready card");
        let age = g.world.tick.saturating_sub(card.tick) / 30;
        assert_eq!(card.caption(age), format!("HOOK READY {age}S AGO"));
    }

    #[test]
    fn a_site_without_a_builder_raises_a_card_until_a_builder_returns() {
        let mut g = game();
        let site = g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(24, 44));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == site) {
            e.build_remaining = 600;
            e.builder = None;
        }
        settle(&mut g, 60);
        let card = |g: &Game| {
            g.ux.alerts
                .entries
                .iter()
                .find(|a| a.kind == AlertKind::NoBuilder(Kind::Drydock))
                .cloned()
        };
        assert!(card(&g).is_none(), "a short gap is not a stall");
        settle(&mut g, 8 * 30);
        let raised = card(&g).expect("the no-builder card");
        assert!(raised.kind.urgent());
        assert!(
            raised.caption(0).starts_with("NO BUILDER: DRYDOCK SITE"),
            "{}",
            raised.caption(0)
        );
        g.render();
        keep_frame(&g, "no-builder-card");
        // It stays while the site waits, past a red card's life.
        settle(&mut g, 45 * 30);
        assert!(card(&g).is_some(), "the card waits with the site");
        // A worker sent to it clears the card.
        let worker = own(&g, Kind::Hook);
        g.issue(bw_sim::Command::Repair {
            units: vec![worker],
            target: site,
        });
        settle(&mut g, 10);
        assert!(card(&g).is_none(), "a builder on its way clears the card");
    }

    #[test]
    fn tide_buttons_say_what_opens_and_that_deep_is_a_wall() {
        let g = game();
        for north in [true, false] {
            let words = g.action_description(&Action::SetTide(bw_sim::Arm::from_north(north)));
            assert!(words.contains("a wall"), "{words}");
            assert!(words.contains("to walking machines"), "{words}");
        }
        let flood = g.action_description(&Action::Flood);
        assert!(flood.contains("a wall"), "{flood}");
        let lane_wrecks = g
            .world
            .map
            .resources
            .iter()
            .filter(|r| {
                let (x, y) = r.pos.cell_xy();
                r.remaining > 0
                    && g.world.map.terrain(x, y).tidal_arm().map(|arm| arm == 0) == Some(true)
            })
            .count();
        if lane_wrecks > 0 && !g.world.gate.north_dry() {
            let words = g.action_description(&Action::SetTide(bw_sim::Arm::NORTH));
            assert!(words.contains("salvage) to workers"), "{words}");
        }
    }

    #[test]
    fn full_pressure_names_reclaim_and_the_header_says_spend() {
        let mut g = game();
        g.world.players[0].pressure = g.world.players[0].pressure_cap;
        settle(&mut g, 70);
        assert!(g.message.contains("RECLAIM"), "{}", g.message);
        g.render();
        keep_frame(&g, "pressure-full");
    }

    #[test]
    fn gather_words_name_the_nearest_drop() {
        let mut g = game();
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.pos)
            .expect("hq");
        let far = g
            .world
            .map
            .resources
            .iter()
            .max_by_key(|r| r.pos.distance_sq(hq))
            .map(|r| (r.id, r.pos))
            .expect("a wreck");
        let near = g
            .world
            .map
            .resources
            .iter()
            .min_by_key(|r| r.pos.distance_sq(hq))
            .map(|r| r.id)
            .expect("a wreck");
        assert!(g.gather_words(near).contains("headquarters"));
        let (x, y) = far.1.cell_xy();
        g.world
            .spawn_for_tests(0, Kind::Dropoff, Pos::cell(x + 3, y));
        assert!(
            g.gather_words(far.0).contains("salvage yard"),
            "{}",
            g.gather_words(far.0)
        );
    }

    #[test]
    fn the_sluice_card_folds_and_keeps_the_checklist() {
        let mut g = game();
        g.ux.preferences.tide_card = true;
        let s = g.ui_scale();
        g.render();
        let open = g.route_bounds();
        keep_frame(&g, "field-cards-open");
        g.action(Action::CompactFieldCards);
        g.render();
        keep_frame(&g, "field-cards-folded");
        let folded = g.route_bounds();
        assert!(
            folded.h <= 36 * s && folded.h < open.h,
            "{} vs {}",
            folded.h,
            open.h
        );
        assert!(
            g.buttons
                .iter()
                .any(|b| b.action == Action::FocusHold && b.label == "HOLD CHECKLIST"),
            "the checklist stays"
        );
        // The fold stays across a new match, and unfolds again.
        assert!(g.ux.preferences.compact_field_cards);
        let toggle = g
            .buttons
            .iter()
            .find(|b| b.action == Action::CompactFieldCards)
            .cloned()
            .expect("the fold button");
        assert!(folded.contains(toggle.x, toggle.y));
        g.action(Action::CompactFieldCards);
        g.render();
        assert_eq!(g.route_bounds().h, open.h);
        assert_eq!(s, 2);
    }

    #[test]
    fn a_prompt_message_belongs_to_the_selection_it_was_about() {
        let mut g = game();
        g.selected = vec![own(&g, Kind::Headquarters)];
        g.notify("Not enough salvage.");
        assert_eq!(g.native_prompt(), "Not enough salvage.");
        g.selected = vec![own(&g, Kind::Hook)];
        assert!(!g.message_live(), "a new selection clears it");
        assert_ne!(g.native_prompt(), "Not enough salvage.");
    }

    #[test]
    fn tooltips_are_smaller_than_the_selection_panel() {
        let mut g = game();
        let s = g.ui_scale();
        g.selected = vec![own(&g, Kind::Headquarters)];
        g.render();
        let train = g
            .buttons
            .iter()
            .find(|b| matches!(b.action, Action::Train(_)))
            .cloned()
            .expect("train");
        g.pointer_moved(train.x + 4, train.y + 4);
        g.render();
        keep_frame(&g, "tooltip");
        let tip = g.native_tooltip_bounds().expect("a tooltip");
        assert_eq!(tip.w, crate::native_ui::TIP_W * s);
        assert!(tip.w * tip.h < 360 * s * 100 * s, "{}x{}", tip.w, tip.h);
        assert!(tip.y + tip.h <= train.y, "above the card");
    }
}
