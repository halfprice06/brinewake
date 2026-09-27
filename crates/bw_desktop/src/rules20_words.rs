//! Rules 20 in the interface: a flood's frozen count on the hold banner and
//! the sluice card, the hold's length for the match (90 s for two seats, 75
//! s for three, 120 s once a seat is out) in the banner, the heard lines
//! and the Guide, and the nest that blocks but does not hold.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};
    use crate::menus::guide_page_on;
    use crate::tide_cues::TideCues;
    use bw_core::{Faction, Kind, Pos, TICK_HZ};
    use bw_sim::{MapId, Tide, World};
    use std::path::PathBuf;

    fn game_on(map: MapId) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-rules20-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = Faction::Union;
        game.map = map;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn banner(g: &Game) -> Option<String> {
        g.buttons
            .iter()
            .find(|b| b.action == Action::FocusHold && b.label != "HOLD CHECKLIST")
            .map(|b| b.label.clone())
    }

    /// Frames for a look by eye: set BW_RULES20_FRAMES to a folder.
    fn keep(g: &Game, name: &str) {
        if let Ok(dir) = std::env::var("BW_RULES20_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&PathBuf::from(dir).join(format!("{name}.png")))
                .expect("frame");
        }
    }

    fn hold_everything(world: &mut World, seat: u8) {
        world.gate.owner = Some(seat);
        for arm in world.arms_of(seat) {
            let mouth = world.own_mouth(seat, arm).expect("own bank");
            world.spawn_for_tests(seat, Kind::Riveter, mouth);
        }
    }

    fn knock_out(world: &mut World, seat: u8) {
        world
            .entities
            .retain(|e| !(e.owner == seat && e.kind == Kind::Headquarters));
        world.step();
        assert!(world.is_eliminated(seat));
    }

    #[test]
    fn a_flood_shows_the_count_frozen_on_the_banner_and_the_card() {
        let mut g = game_on(MapId::SplitBasin);
        hold_everything(&mut g.world, 0);
        g.world.lane_hold[0] = 40 * 30;
        assert_eq!(g.flood_status(300), "FROZEN: FLOOD 10S");
        g.world.gate.tide = Tide::Flood;
        g.world.gate.flood_until = Some(g.world.tick + 20 * 30);
        g.tick();
        g.render();
        assert!(g.world.hold_frozen());
        assert_eq!(g.world.lane_hold[0], 40 * 30, "the count stands still");
        assert_eq!(banner(&g).as_deref(), Some("FROZEN: FLOOD"));
        keep(&g, "frozen-own-count");
        // With no count banked the card just counts the flood down.
        g.world.lane_hold[0] = 0;
        g.world.gate.owner = None;
        assert_eq!(g.flood_status(300), "FLOOD 10S");

        // An opponent's count on the Confluence freezes the same way.
        let mut g = game_on(MapId::Confluence);
        hold_everything(&mut g.world, 2);
        g.world.lane_hold[2] = 50 * 30;
        g.world.gate.tide = Tide::Flood;
        g.world.gate.flood_until = Some(g.world.tick + 20 * 30);
        g.tick();
        g.render();
        assert_eq!(g.world.lane_hold[2], 50 * 30);
        assert_eq!(banner(&g).as_deref(), Some("FROZEN: FLOOD"));
        keep(&g, "frozen-enemy-count");
    }

    #[test]
    fn the_banner_states_the_hold_length_for_the_match() {
        let mut g = game_on(MapId::SplitBasin);
        g.world.gate.owner = Some(0);
        g.world.lane_hold[0] = 40 * 30;
        g.tick();
        g.render();
        let two = banner(&g).expect("banner");
        assert!(two.starts_with("YOUR 39/90"), "{two}");

        let mut g = game_on(MapId::Confluence);
        g.world.gate.owner = Some(0);
        g.world.lane_hold[0] = 40 * 30;
        g.tick();
        g.render();
        let three = banner(&g).expect("banner");
        assert!(three.starts_with("YOUR 39/75"), "{three}");
        keep(&g, "three-seat-75");
        knock_out(&mut g.world, 2);
        g.world.gate.owner = Some(0);
        g.tick();
        g.render();
        let after = banner(&g).expect("banner");
        assert!(
            after.starts_with("YOUR ") && after.contains("/120"),
            "{after}"
        );
        keep(&g, "seat-out-120");
        // The one lane left to hold is the only one asked for.
        let live = g.world.hold_arms(0);
        assert_eq!(live.len(), 1);
        let mouth = g.world.own_mouth(0, live[0]).expect("own bank");
        g.world.spawn_for_tests(0, Kind::Riveter, mouth);
        assert!(g.world.holds_every_lane(0));
        let letter = crate::seats::arm_letter(&g.world, live[0]);
        assert_eq!(g.crossing_held(), Some(format!("YOU HOLD {letter} LANE")));
    }

    #[test]
    fn the_heard_line_names_seventy_five_seconds_with_three_seats() {
        let mut world = World::with_map(
            21,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("three seats");
        world.ai_enabled = false;
        hold_everything(&mut world, 0);
        let mut cues = TideCues::default();
        for _ in 0..(TICK_HZ as usize * 2) {
            world.step();
            cues.observe(&world);
        }
        let heard: Vec<String> = cues.recent().map(|h| h.text.clone()).collect();
        assert!(
            heard.iter().any(|t| t == "you hold both lanes: 75s to win"),
            "{heard:?}"
        );
    }

    #[test]
    fn the_guide_states_the_rules_20_hold_and_income() {
        let (three, _) = guide_page_on(3, Faction::Union, MapId::Confluence);
        let text = three.join(" ");
        // Rules 22: with three seats a nest slows a count instead.
        for words in ["NESTS SLOW A COUNT", "75S", "COUNTS FREEZE"] {
            assert!(text.contains(words), "{words}: {text}");
        }
        assert!(!text.contains("90S"), "{text}");
        let (two, _) = guide_page_on(3, Faction::Union, MapId::SplitBasin);
        let text = two.join(" ");
        for words in ["NESTS BLOCK, BUT DON'T HOLD", "90S", "COUNTS FREEZE"] {
            assert!(text.contains(words), "{words}: {text}");
        }
        let (economy, _) = guide_page_on(1, Faction::Union, MapId::SplitBasin);
        assert!(
            economy.iter().any(|l| l.contains("40 SALVAGE A MINUTE")),
            "{economy:?}"
        );
        let (basics, _) = guide_page_on(0, Faction::Union, MapId::Confluence);
        assert!(basics.iter().any(|l| l.contains("TWO LANES 75S")));
        assert!(basics.iter().any(|l| l.contains("OUT: 120S")));
    }

    #[test]
    fn a_nest_at_the_far_mouth_shows_as_a_foe_that_blocks_the_lane() {
        let mut g = game_on(MapId::SplitBasin);
        hold_everything(&mut g.world, 0);
        assert!(g.world.holds_lane(0, 0));
        let far = g.world.crossing_mouths()[0][1];
        g.world.spawn_for_tests(1, Kind::Tower, far);
        assert!(!g.world.holds_lane(0, 0));
        let holders = crate::qol::mouth_holders(&g.world);
        assert!(holders[0][1].1, "the nest reads as a foe at that mouth");
        let (own, enemy) = crate::qol::lane_presence(&g.world, 0);
        assert!(own);
        assert_eq!(enemy, Some(far));
        // Our own nest is not a foe, and it holds nothing.
        let near = g.world.crossing_mouths()[1][0];
        g.world.spawn_for_tests(
            0,
            Kind::Tower,
            Pos::cell(near.cell_xy().0, near.cell_xy().1),
        );
        assert!(!crate::qol::mouth_holders(&g.world)[1][0].1);
    }
}
