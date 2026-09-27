//! Rules 21 in the interface: the headquarters' OVERHAUL button, its
//! tooltip and the Guide's TECH line; the drain rate in the Guide and the
//! hold card; the Glinter's bonus against a deployed Loom in its roster
//! line.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Button, Game};
    use crate::menus::guide_page_on;
    use bw_content::{TIDE_HOLD_DRAIN_PER_TICK, Upgrade};
    use bw_core::{Faction, Kind};
    use bw_sim::MapId;
    use std::path::PathBuf;

    fn game_on(faction: Faction, map: MapId) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-rules21-{}-{:?}",
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

    /// Frames for a look by eye: set BW_RULES21_FRAMES to a folder.
    fn keep(g: &Game, name: &str) {
        if let Ok(dir) = std::env::var("BW_RULES21_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&PathBuf::from(dir).join(format!("{name}.png")))
                .expect("frame");
        }
    }

    fn own_hq(g: &Game) -> u32 {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .expect("own headquarters")
    }

    fn overhaul_button(g: &Game) -> Button {
        g.buttons
            .iter()
            .find(|b| matches!(b.action, Action::Upgrade(u) if u.overhaul_level().is_some()))
            .cloned()
            .expect("an OVERHAUL button on the headquarters card")
    }

    fn select_hq(g: &mut Game) {
        g.selected = vec![own_hq(g)];
        g.tick();
        g.render();
    }

    #[test]
    fn the_headquarters_card_offers_overhaul_level_by_level() {
        for (faction, map) in [
            (Faction::Union, MapId::SplitBasin),
            (Faction::Assembly, MapId::Confluence),
            (Faction::Compact, MapId::Confluence),
        ] {
            let mut g = game_on(faction, map);
            g.world.players[0].salvage = 5_000;
            select_hq(&mut g);
            let b = overhaul_button(&g);
            assert_eq!(b.action, Action::Upgrade(Upgrade::Overhaul1));
            assert!(b.enabled, "{faction:?}: affordable");
            assert!(b.hint.contains("1000S"), "{:?}", b.hint);
            assert!(b.hint.ends_with("HULL +6%"), "{:?}", b.hint);
            assert!(crate::lean_hud::icon_key(&b.action, faction).is_some());
            keep(&g, &format!("hq-card-overhaul-{faction:?}"));

            // The tooltip: salvage and time, no pressure, and the effect.
            let card = g.hover_card(&b).expect("a card");
            assert_eq!(card.title, "OVERHAUL I");
            let costs: Vec<String> = card.costs.iter().map(|c| c.value.clone()).collect();
            assert_eq!(costs, vec!["1000".to_string(), "45S".to_string()]);
            let body = card.body.join(" ");
            assert!(body.contains("6% more hull"), "{body}");
            assert!(body.contains("built or not"), "{body}");

            // P buys it; the button then counts down and offers level II.
            g.key("P", false, false);
            g.tick();
            g.render();
            let b = overhaul_button(&g);
            assert_eq!(b.action, Action::Upgrade(Upgrade::Overhaul2));
            assert!(b.hint.ends_with("S LEFT"), "{:?}", b.hint);
            keep(&g, &format!("hq-card-overhaul-running-{faction:?}"));
            // K at the headquarters cancels it for 75%.
            let before = g.world.players[0].salvage;
            g.key("K", false, false);
            g.tick();
            assert!(g.world.players[0].salvage >= before + 750);
            let hq = own_hq(&g);
            assert!(
                g.world
                    .entities
                    .iter()
                    .any(|e| e.id == hq && e.upgrade.is_none())
            );
        }
    }

    #[test]
    fn a_later_level_says_it_needs_the_one_before() {
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        g.world.players[0].salvage = 5_000;
        select_hq(&mut g);
        let reason = g
            .action_reason(&Action::Upgrade(Upgrade::Overhaul2))
            .expect("refused");
        assert_eq!(reason, "OVERHAUL II needs OVERHAUL I first.");
        g.world.players[0].upgrades = Upgrade::OVERHAUL.to_vec();
        g.render();
        let b = overhaul_button(&g);
        assert!(b.hint.ends_with("DONE"), "{:?}", b.hint);
        assert!(!b.enabled);
        keep(&g, "hq-card-overhaul-done");
    }

    #[test]
    fn the_guide_states_overhaul_and_the_drain_rate() {
        assert_eq!(TIDE_HOLD_DRAIN_PER_TICK, 3, "the Guide says 3S A SECOND");
        for map in MapId::ALL {
            let (sluice, _) = guide_page_on(3, Faction::Union, map);
            let text = sluice.join(" ");
            assert!(text.contains("A BROKEN COUNT DRAINS 3S A SECOND"), "{text}");
            for faction in Faction::ALL {
                let (tech, keys) = guide_page_on(6, faction, map);
                let text = tech.join(" ");
                assert!(text.contains("OVERHAUL AT THE HQ"), "{text}");
                assert!(text.contains("1000/1500/2000S"), "{text}");
                assert!(text.contains("+6% HULL"), "{text}");
                assert!(
                    keys.iter()
                        .any(|(k, a)| *k == "P" && a.contains("OVERHAUL"))
                );
            }
        }
    }

    #[test]
    fn the_hold_card_states_the_drain_rate() {
        let mut g = game_on(Faction::Union, MapId::SplitBasin);
        g.world.lane_hold[0] = 40 * 30;
        g.tick();
        let brief = g.hold_brief();
        assert!(brief.contains("drains 3s a second"), "{brief}");
        let mut g = game_on(Faction::Union, MapId::Confluence);
        g.world.lane_hold[1] = 40 * 30;
        g.tick();
        let brief = g.hold_brief();
        assert!(brief.contains("drains 3s a second"), "{brief}");
    }

    #[test]
    fn the_glinter_hunts_looms_as_its_roster_says() {
        assert!(crate::ux::role_description(Kind::Glinter).contains("+5 against a deployed Loom"));
        assert!(crate::ux::answers(Kind::Loom, Faction::Compact).contains(&Kind::Glinter));
    }
}
