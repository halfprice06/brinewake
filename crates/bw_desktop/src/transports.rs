//! Tests for the transports' interface: boarding by right-click, the
//! UNLOAD button and key, the hold readout and the Drydock's fourth key.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game};
    use bw_core::{Faction, Kind, Pos};
    use bw_sim::Command;
    use std::path::PathBuf;

    fn game(faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new(base);
        game.faction = faction;
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

    #[test]
    fn a_right_click_on_an_own_lifter_boards_and_u_unloads() {
        let mut g = game(Faction::Union);
        let lifter = g.world.spawn_for_tests(0, Kind::Lifter, Pos::cell(30, 60));
        let riveter = g.world.spawn_for_tests(0, Kind::Riveter, Pos::cell(28, 60));
        g.selected = vec![riveter];
        g.camera.center(Pos::cell(30, 60));
        g.render();
        let (x, y) = g.project(Pos::cell(30, 60));
        let order = g.context_order(x, y, false).map(|(c, _, _)| c);
        assert!(
            matches!(order, Some(Command::Board { .. })),
            "a right-click on the transport boards: {order:?}"
        );
        g.right_click(x, y, false);
        assert!(g.message.starts_with("Board order"), "{}", g.message);
        settle(&mut g, 120);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.id == riveter)
                .unwrap()
                .aboard,
            Some(lifter)
        );
        // The rider is off the field: not drawn, not picked, not in the army.
        g.selected.clear();
        g.render();
        assert!(!g.army_ids().contains(&riveter));
        g.selected = vec![lifter];
        g.render();
        let unload = g.buttons.iter().find(|b| b.action == Action::Unload);
        assert!(unload.is_some(), "the transport's card offers UNLOAD");
        let state = g.console_state();
        assert!(
            state
                .selection
                .as_ref()
                .is_some_and(|s| s.status.starts_with("HOLD 1/4")),
            "{:?}",
            state.selection.map(|s| s.status)
        );
        g.key("U", false, false);
        assert_eq!(g.message, "Unload order sent.");
        settle(&mut g, 5);
        assert!(
            g.world
                .entities
                .iter()
                .find(|e| e.id == riveter)
                .unwrap()
                .aboard
                .is_none()
        );
        assert!(g.army_ids().contains(&riveter));
    }

    #[test]
    fn the_drydock_trains_the_transport_on_r() {
        let mut g = game(Faction::Assembly);
        g.world.players[0].salvage = 2_000;
        g.world.players[0].pressure = 300;
        let drydock = g.world.spawn_for_tests(0, Kind::Drydock, Pos::cell(8, 60));
        g.selected = vec![drydock];
        g.render();
        assert!(
            g.buttons
                .iter()
                .any(|b| b.label == "BARGE" && b.hint.starts_with('R')),
            "{:?}",
            g.buttons
                .iter()
                .map(|b| b.label.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(g.production_hotkey_for(Kind::Barge), Some("R"));
        g.key("R", false, false);
        settle(&mut g, 5);
        let queued = g
            .world
            .entities
            .iter()
            .find(|e| e.id == drydock)
            .map(|e| e.queue.iter().any(|p| p.kind == Kind::Barge))
            .unwrap_or(false);
        assert!(queued, "R queues the Barge at a Drydock by the water");
    }
}
