//! Tests for the fixes from the seventh two-agent trial: the sluice card and
//! its tide buttons act on release so a drag over them boxes the ground, and
//! wells and wreck beds are findable without hunting for them.
#[cfg(test)]
mod tests {
    use crate::game::{Action, Game, Mode};
    use bw_core::Kind;
    use bw_sim::Command;
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

    #[test]
    fn the_tide_buttons_act_on_release_and_a_drag_over_the_card_boxes_the_ground() {
        let mut g = game();
        g.world.gate.owner = Some(0);
        g.world.players[0].pressure = 200;
        g.render();
        let dry = g
            .buttons
            .iter()
            .find(|b| b.action == Action::SetTide(bw_sim::Arm::NORTH))
            .cloned()
            .expect("the DRY N button");
        let (cx, cy) = (dry.x + dry.w / 2, dry.y + dry.h / 2);
        let before = g.world.command_log.len();
        g.left_down(cx, cy);
        assert_eq!(
            g.world.command_log.len(),
            before,
            "the press alone does not spend 40 pressure"
        );
        g.left_up(cx, cy, false);
        assert!(
            matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(Command::SetTide {
                    arm: bw_sim::Arm::NORTH
                })
            ),
            "{:?}",
            g.world.command_log.last().map(|r| &r.command)
        );
        // A press that becomes a drag never fires the button.
        let before = g.world.command_log.len();
        g.left_down(cx, cy);
        g.left_up(cx + 80, cy + 60, false);
        assert_eq!(g.world.command_log.len(), before);

        // The card body looks at the station on release, not on the press,
        // and a drag from it boxes the ground instead of jumping the camera.
        let card = g.route_bounds();
        let (px, py) = (card.x + 8, card.y + card.h - 4);
        let home = (g.camera.x, g.camera.y);
        g.left_down(px, py);
        assert_eq!(
            (g.camera.x, g.camera.y),
            home,
            "the press does not move the camera"
        );
        g.left_up(px + 90, py + 70, false);
        assert_eq!(
            (g.camera.x, g.camera.y),
            home,
            "a drag from the card leaves the camera where it was"
        );
        g.left_down(px, py);
        g.left_up(px, py, false);
        assert_ne!(
            (g.camera.x, g.camera.y),
            home,
            "a click on the card still looks at the station"
        );
    }

    #[test]
    fn a_pending_order_reaches_the_ground_under_the_card() {
        let mut g = game();
        g.render();
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
        // Trial 10: while a placement waits the card stands aside, so the
        // ground under it takes the click.
        let card = g.tide_card_rect();
        g.render();
        assert_eq!(g.route_bounds().h, 0, "the card stands aside");
        let (x, y) = (card.x + 8, card.y + card.h - 4);
        assert!(g.world_pointer_allowed(x, y));
        g.left_down(x, y);
        assert!(
            !g.message.contains("sluice card"),
            "the click reached the ground: {}",
            g.message
        );
    }

    #[test]
    fn wells_are_named_under_the_pointer_and_wreck_beds_read_on_the_chart() {
        let mut g = game();
        let well = g.world.map.wells[0];
        g.camera.center(well);
        g.cursor = (-999, -999);
        g.render();
        assert!(g.hover_tip.is_none(), "no words at rest");
        // A well names itself under the pointer (playtest, 2026-09-23: the
        // WELL plate was too big and covered the art).
        let (px, py) = g.project(well);
        g.cursor = (px, py - 4);
        g.render();
        let tip = g.hover_tip.clone().expect("a card under the pointer");
        assert_eq!(tip.card.title, "WELL");
        assert!(tip.anchor.1 < py, "the card stands over the well");

        // The chart names every wreck bed that still holds salvage, seen or
        // not: both seats planned gathering blind without it.
        let bed = g
            .world
            .map
            .resources
            .iter()
            .find(|r| r.remaining > 0 && !g.world.visible(0, r.pos))
            .expect("an unseen wreck");
        let r = g.minimap_bounds();
        let chart = crate::minimap_chart::ChartRect::field(r.x, r.y, r.w, r.h);
        let (bx, by) = chart.plot(&g.world, bed.pos);
        let marked = (by - 2..=by + 2).any(|y| {
            (bx - 2..=bx + 2).any(|x| g.canvas.get(x, y) == Some(crate::minimap_chart::ROCK))
        });
        assert!(marked, "the wreck bed is inked on the chart at {bx},{by}");
    }

    #[test]
    fn a_mouth_held_without_the_station_says_what_is_missing() {
        let mut g = game();
        for mouths in bw_sim::CROSSING_MOUTHS {
            let (x, y) = mouths[0].cell_xy();
            g.world
                .spawn_for_tests(0, Kind::Riveter, bw_core::Pos::cell(x, y));
        }
        g.world.gate.owner = None;
        g.render();
        assert!(!g.world.holds_lane(0, 0), "no station, no hold");
        assert_eq!(g.hold_gauge(), None);
        assert_eq!(
            g.crossing_held().as_deref(),
            Some("TAKE THE SLUICE TO HOLD A LANE"),
            "the card says what is missing"
        );
        g.world.gate.owner = Some(0);
        g.render();
        assert!(g.world.holds_lane(0, 0) && g.world.holds_lane(0, 1));
    }

    #[test]
    fn the_guide_names_the_station_in_the_hold_rule() {
        let (lines, _) = crate::menus::guide_page(3, bw_core::Faction::Union);
        assert!(lines[0].contains("OWN THE SLUICE"), "{}", lines[0]);
        assert!(
            lines[1].contains("NO ENEMY GUN OR NEST AT EITHER"),
            "{}",
            lines[1]
        );
    }
}
