//! Tests for the tenth trial's words (fix stream S2): the three-arm tide
//! prompts, the Guide on the Confluence, the COMBAT page for every side,
//! counters and upgrades in the reader's own terms, the doctrine blocker,
//! the Hauling and Surge prices, the hull a plated or refitted machine
//! shows, and the Compact's own building names.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Button, Game, Screen};
    use crate::menus::{GUIDE_PAGES, MenuModel, guide_page_on, roster_machines, roster_train};
    use bw_content::{Doctrine, Upgrade};
    use bw_core::{Faction, Kind};
    use bw_sim::MapId;
    use std::path::PathBuf;

    fn game_on(faction: Faction, map: MapId) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-t10words-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.map = map;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn button(action: Action) -> Button {
        Button {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
            label: String::new(),
            hint: String::new(),
            action,
            enabled: true,
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

    #[test]
    fn a_counter_never_names_its_own_sides_machines() {
        // Trial 10: the Reedguard "lost to Looms", its own side's gun.
        for faction in Faction::ALL {
            let own = roster_machines(faction);
            for kind in &own {
                let (strong, weak) = crate::ux::matchups(*kind);
                for other in &own {
                    for word in [other.name().to_string(), format!("{}S", other.name())] {
                        for line in [strong, weak] {
                            assert!(
                                !line
                                    .split(|c: char| !c.is_ascii_alphanumeric())
                                    .any(|w| w == word),
                                "{kind:?} names its own {other:?}: {line}"
                            );
                        }
                    }
                }
            }
        }
        // And an enemy's machine says what of yours answers it.
        assert!(crate::ux::answers(Kind::Loom, Faction::Union).contains(&Kind::Sounder));
        // Rules 22: the Brander, not the Heliostat, beats Reedguards in a
        // duel for equal salvage.
        assert!(crate::ux::answers(Kind::Reedguard, Faction::Compact).contains(&Kind::Brander));
        assert!(crate::ux::answers(Kind::Heliostat, Faction::Assembly).contains(&Kind::Loom));
    }

    #[test]
    fn upgrades_speak_only_of_the_readers_own_machines() {
        for faction in Faction::ALL {
            let plate = crate::ux::upgrade_description(Upgrade::Plate, faction);
            for kind in roster_machines(faction) {
                let named = plate.to_uppercase().contains(kind.name());
                assert_eq!(named, bw_sim::plated(kind), "{faction:?} {kind:?}: {plate}");
            }
            let siege = crate::ux::upgrade_description(Upgrade::Siege, faction);
            assert!(!siege.contains("Union:") && !siege.contains("Assembly:"));
            // Every upgrade a Works offers does something for every side.
            for upgrade in Upgrade::offered_by(Kind::Works) {
                assert!(
                    !crate::ux::upgrade_machines(*upgrade, faction).is_empty(),
                    "{faction:?} {upgrade:?}"
                );
            }
        }
        assert!(
            crate::ux::upgrade_description(Upgrade::Plate, Faction::Compact)
                .starts_with("Branders and Heliostats")
        );
        assert!(
            crate::ux::upgrade_description(Upgrade::Siege, Faction::Compact).contains("Heliostats")
        );
        assert!(crate::ux::upgrade_hint(Upgrade::Siege, Faction::Compact).ends_with("BEAM X2"));
        // The hover card reads the player's own line.
        let g = game_on(Faction::Compact, MapId::SplitBasin);
        let card = g
            .hover_card(&button(Action::Upgrade(Upgrade::Plate)))
            .expect("card");
        let body = card.body.join(" ");
        assert!(body.contains("Branders"), "{body}");
        assert!(!body.contains("Bulwarks"), "{body}");
        assert_eq!(card.tag.as_deref(), Some("UPGRADE / GLASSWORKS"));
    }

    #[test]
    fn a_shut_doctrine_says_the_two_exclude_each_other() {
        let mut g = game_on(Faction::Assembly, MapId::SplitBasin);
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        g.world.players[0].doctrine = Some(Doctrine::Hauling);
        for tier in [1, 2] {
            g.world.players[0].doctrine_tier = tier;
            let reason = g
                .action_reason(&Action::Research(Doctrine::FireControl))
                .expect("shut");
            assert_eq!(reason, "HAULING chosen: the doctrines exclude each other.");
        }
        let reason = g.action_reason(&Action::Research(Doctrine::Hauling));
        assert_eq!(
            reason.as_deref(),
            Some("HAULING is complete at both tiers.")
        );
    }

    #[test]
    fn hauling_names_its_pause_and_its_second_tier() {
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        g.world.players[0].salvage = 1000;
        g.world.players[0].pressure = 500;
        let card = g
            .hover_card(&button(Action::Research(Doctrine::Hauling)))
            .expect("card");
        assert_eq!(card.title, "HAULING");
        let body = card.body.join(" ");
        assert!(body.contains("Pauses HQ worker training 30s."), "{body}");
        g.world.players[0].doctrine = Some(Doctrine::Hauling);
        g.world.players[0].doctrine_tier = 1;
        let card = g
            .hover_card(&button(Action::Research(Doctrine::Hauling)))
            .expect("card");
        assert_eq!(card.title, "HAULING II");
        assert!(card.body.join(" ").contains("45s"), "{:?}", card.body);
    }

    #[test]
    fn surge_shows_its_price_before_the_press() {
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        let home = g.world.start_of(0);
        let ids: Vec<u32> = (0..3)
            .map(|i| {
                g.world.spawn_for_tests(
                    0,
                    Kind::Riveter,
                    bw_core::Pos::cell(home.x / bw_core::FP + 6 + i, home.y / bw_core::FP + 6),
                )
            })
            .collect();
        g.selected = ids;
        let card = g.hover_card(&button(Action::Surge)).expect("card");
        assert_eq!(
            card.body.join(" "),
            "10 P per machine: 3 of 3 surge = 30 P."
        );
        assert!(
            g.action_description(&Action::Surge)
                .starts_with("10 P per machine: 3 of 3 surge = 30 P.")
        );
    }

    #[test]
    fn a_refitted_machine_shows_its_real_hull() {
        // Trial 10: a Loom read 184/160 after REFIT.
        let mut g = game_on(Faction::Assembly, MapId::SplitBasin);
        let home = g.world.start_of(0);
        let loom = g.world.spawn_for_tests(
            0,
            Kind::Loom,
            bw_core::Pos::cell(home.x / bw_core::FP + 6, home.y / bw_core::FP + 6),
        );
        let e = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == loom)
            .expect("loom");
        e.max_hp = 184;
        e.hp = 184;
        g.selected = vec![loom];
        let selection = g.console_state().selection.expect("selection");
        assert_eq!((selection.hp, selection.max_hp), (184, 184));
    }

    #[test]
    fn the_compact_calls_its_buildings_by_their_own_names() {
        let mut g = game_on(Faction::Compact, MapId::SplitBasin);
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        let selection = g.console_state().selection.expect("selection");
        assert_eq!(selection.title, "KILN");
        let card = g
            .hover_card(&button(Action::Build(Kind::Works)))
            .expect("card");
        assert_eq!(card.title, "GLASSWORKS");
        assert_eq!(
            crate::ux::building_name(Kind::Dropoff, Faction::Compact),
            "RAKE SHED"
        );
        // The other sides keep the names they had.
        for faction in [Faction::Union, Faction::Assembly] {
            assert_eq!(crate::ux::building_name(Kind::Works, faction), "WORKS");
        }
    }

    #[test]
    fn the_guide_on_the_confluence_names_three_arms_and_the_hold_rule() {
        let (lines, keys) = guide_page_on(3, Faction::Union, MapId::Confluence);
        let text = lines.join(" ");
        assert!(text.contains("DRY E / W / S"), "{text}");
        assert!(!text.contains("DRY N"), "{text}");
        // Rules 22: with three seats a nest slows a count.
        assert!(text.contains("NESTS SLOW A COUNT"), "{text}");
        assert!(text.contains("A BROKEN COUNT DRAINS 3S A SECOND"), "{text}");
        assert!(
            keys.iter()
                .any(|(k, a)| *k == "T" && a.contains("NEXT LANE"))
        );
        let (basin, _) = guide_page_on(3, Faction::Union, MapId::SplitBasin);
        assert!(basin.iter().any(|l| l.contains("DRY N / DRY S")));
        assert!(
            basin
                .iter()
                .any(|l| l.contains("NESTS BLOCK, BUT DON'T HOLD"))
        );
        let (economy, _) = guide_page_on(1, Faction::Union, MapId::Confluence);
        assert!(economy.iter().any(|l| l.contains("UP TO 90")));
        // Every page's facts fit the page, on both maps and for every side.
        for map in MapId::ALL {
            for faction in Faction::ALL {
                for page in 0..GUIDE_PAGES {
                    if page == 4 || page == crate::menus::ROSTER_PAGE {
                        continue;
                    }
                    let (lines, keys) = guide_page_on(page, faction, map);
                    assert!((3..=4).contains(&lines.len()), "{page}");
                    assert!(keys.len() <= 8);
                    for line in lines {
                        assert!(crate::canvas::text_readable_width(line) <= 544, "{line}");
                    }
                    for (key, action) in keys {
                        assert!(crate::canvas::text_readable_width(key) <= 72, "{key}");
                        assert!(
                            crate::canvas::text_readable_width(action) <= 196,
                            "{action}"
                        );
                    }
                }
            }
        }
        // The TECH page says U and J are two doctrines, not one then II.
        for faction in Faction::ALL {
            let (lines, keys) = guide_page_on(6, faction, MapId::Confluence);
            assert!(lines[0].contains("HAULING (U) OR FIRE CONTROL (J), NEVER BOTH"));
            assert!(!keys.iter().any(|(k, _)| *k == "U / J"));
        }
        // The sluice page draws the three-arm chart on the Confluence.
        let mut canvas = crate::canvas::Canvas::default();
        let model = MenuModel {
            help_page: 3,
            selected_map: MapId::Confluence,
            ..MenuModel::default()
        };
        crate::menus::help(&mut canvas, None, &model);
        let mut basin = crate::canvas::Canvas::default();
        crate::menus::help(
            &mut basin,
            None,
            &MenuModel {
                help_page: 3,
                ..MenuModel::default()
            },
        );
        assert_ne!(canvas.pixels, basin.pixels, "a different chart");
        keep(&canvas, "guide-sluice-confluence");
    }

    #[test]
    fn the_combat_page_opens_on_your_own_specialist() {
        let mut g = game_on(Faction::Compact, MapId::Confluence);
        g.action(Action::Help);
        assert_eq!(g.screen, Screen::Help);
        g.action(Action::GuidePage(4));
        g.render();
        assert_eq!(
            g.ux.manual.subject,
            crate::field_manual::ManualSubject::Heliostat
        );
        for subject in crate::field_manual::ManualSubject::ALL {
            assert!(
                g.buttons
                    .iter()
                    .any(|b| b.action == Action::ManualSubject(subject)),
                "{subject:?}"
            );
        }
        keep(&g.ui_canvas, "guide-combat-compact");
        // Deployed, the beam held on one target for a while.
        g.action(Action::ManualStep);
        g.action(Action::ManualStep);
        g.ux.manual.phase_tick += 60;
        g.render();
        assert_eq!(g.ux.manual.phase(), 2);
        keep(&g.ui_canvas, "guide-combat-compact-beam");
    }

    #[test]
    fn the_roster_names_both_places_a_scout_is_made() {
        // Trial 10: "Stilt: W AT HQ"; since trial 11 it is W at the
        // Drydock too, and the roster says both.
        for faction in Faction::ALL {
            let made = roster_train(faction, faction.scout());
            assert_eq!(made, "W AT HQ\nW AT DRYDOCK", "{faction:?}");
        }
        // Its tooltip names W at both.
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        let hq = own(&g, Kind::Headquarters);
        g.selected = vec![hq];
        assert_eq!(g.production_hotkey_for(Kind::Tidewatch), Some("W"));
    }

    #[test]
    fn a_draining_count_reads_as_a_number_that_falls() {
        // Trial 10: "YOUR COUNT DRAINS" beside 66S, 75S, 89S, rising.
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        g.world.gate.owner = Some(1);
        g.world.lane_hold[1] = 66 * 30;
        let banner = |g: &Game| {
            g.buttons
                .iter()
                .find(|b| b.action == Action::FocusHold && b.label != "HOLD CHECKLIST")
                .map(|b| b.label.clone())
        };
        g.tick();
        g.render();
        assert_eq!(banner(&g).as_deref(), Some("ENEMY DRAINING"));
        keep(&g.canvas, "enemy-draining");
        // Ours: the headline carries the banked seconds, and they fall.
        g.world.gate.owner = Some(0);
        g.world.lane_hold[1] = 0;
        g.world.lane_hold[0] = 40 * 30;
        g.tick();
        g.render();
        let first = banner(&g).expect("banner");
        assert!(first.starts_with("YOUR 39/90"), "{first}");
        // Rules 21: two seconds broken take six from the count.
        for _ in 0..60 {
            g.tick();
        }
        g.render();
        let later = banner(&g).expect("banner");
        assert!(later.starts_with("YOUR 33/90"), "{later}");
        keep(&g.canvas, "own-draining");
    }

    #[test]
    fn another_sides_roster_card_names_your_answer() {
        let mut g = game_on(Faction::Compact, MapId::Confluence);
        g.ux.roster_faction = Some(Faction::Assembly);
        g.open_screen(Screen::Help);
        g.action(Action::GuidePage(crate::menus::ROSTER_PAGE));
        g.action(Action::RosterUnit(Kind::Reedguard));
        g.render();
        keep(&g.ui_canvas, "guide-roster-assembly-for-compact");
        g.ux.roster_faction = Some(Faction::Compact);
        g.action(Action::RosterUnit(Kind::Stilt));
        g.render();
        keep(&g.ui_canvas, "guide-roster-compact");
        g.action(Action::GuidePage(6));
        g.render();
        keep(&g.ui_canvas, "guide-tech-compact");
    }

    /// Keep a frame when `BW_TRIAL10_WORDS_FRAMES` names a folder.
    fn keep(canvas: &crate::canvas::Canvas, name: &str) {
        if let Ok(dir) = std::env::var("BW_TRIAL10_WORDS_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            canvas
                .save(&PathBuf::from(dir).join(format!("{name}.png")))
                .expect("frame");
        }
    }
}
