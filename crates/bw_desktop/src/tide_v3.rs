//! Tests for the tide wall: the neutral start, the open side, the flood and
//! what the interface says about them.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};
    use bw_core::{Kind, Pos};
    use bw_sim::{Command, Depth, Tide};
    use std::path::PathBuf;

    fn game() -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
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

    #[test]
    fn the_card_names_the_tide_and_offers_the_holder_its_choices() {
        let mut g = game();
        g.ux.preferences.tide_card = true;
        assert_eq!(g.world.gate.tide, Tide::Neutral);
        g.render();
        assert!(
            label_of(&g, &Action::Flood).is_none(),
            "nothing to choose while neutral"
        );
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
        assert_eq!(label_of(&g, &Action::Flood).as_deref(), Some("FLOOD"));
        g.action(Action::SetTide(bw_sim::Arm::SOUTH));
        assert!(g.message.contains("south dries"), "{}", g.message);
        settle(&mut g, bw_sim::GATE_WARNING_TICKS as usize + 4);
        assert_eq!(g.world.gate.tide, Tide::Open);
        assert!(!g.world.gate.north_dry());
        g.render();
        assert!(
            label_of(&g, &Action::SetTide(bw_sim::Arm::SOUTH)).is_none(),
            "the open side is not offered"
        );
        assert!(label_of(&g, &Action::SetTide(bw_sim::Arm::NORTH)).is_some());
        let card =
            g.ux.alerts
                .entries
                .iter()
                .find(|a| a.label() == "LANES SWITCHED");
        assert!(card.is_some(), "a card for the changed tide");
    }

    #[test]
    fn shift_t_floods_and_the_cards_and_chart_follow() {
        let mut g = game();
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 300;
        g.key("T", true, false);
        assert!(g.message.starts_with("FLOOD"), "{}", g.message);
        settle(&mut g, bw_content::FLOOD_WARNING_TICKS as usize + 4);
        assert_eq!(g.world.gate.tide, Tide::Flood);
        assert!(
            g.ux.alerts
                .entries
                .iter()
                .any(|a| a.label() == "FLOOD TIDE")
        );
        for (x, y) in [(60, 49), (60, 79), (60, 5), (60, 120)] {
            assert_eq!(g.world.depth_at(x, y), Some(Depth::Deep));
        }
        g.render();
        assert!(
            label_of(&g, &Action::Flood).is_none(),
            "no choice during a flood"
        );
        assert!(
            g.action_reason(&Action::SetTide(bw_sim::Arm::NORTH))
                .is_some()
        );
        settle(&mut g, bw_content::FLOOD_TICKS as usize + 4);
        assert_eq!(
            g.world.gate.tide,
            Tide::Neutral,
            "never opened: the flood falls to neutral"
        );
        assert!(
            g.ux.alerts
                .entries
                .iter()
                .any(|a| a.label() == "TIDE FALLS")
        );
    }

    #[test]
    fn a_machine_cannot_be_sent_into_deep_water_but_walks_the_shallow() {
        let mut g = game();
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(48, 49));
        g.selected = vec![riveter];
        // Neutral: the lane is shallow and open to a slow crossing.
        g.issue(Command::Move {
            units: vec![riveter],
            target: Pos::cell(60, 49),
            queued: false,
        });
        settle(&mut g, 200);
        let pos = g
            .world
            .entities
            .iter()
            .find(|e| e.id == riveter)
            .unwrap()
            .pos;
        assert!(pos.cell_xy().0 > 52, "it entered the shallow lane");
        // Open the south: the north goes deep and the tide sends it back.
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 200;
        g.issue(Command::SetTide {
            arm: bw_sim::Arm::SOUTH,
        });
        settle(&mut g, bw_sim::GATE_WARNING_TICKS as usize + 300);
        let after = g.world.entities.iter().find(|e| e.id == riveter).unwrap();
        assert!(!g.world.swamped(after), "it made for the shore");
        let (ax, ay) = after.pos.cell_xy();
        assert_ne!(g.world.depth_at(ax, ay), Some(Depth::Deep));
    }
}
