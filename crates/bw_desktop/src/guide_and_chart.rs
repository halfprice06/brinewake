//! Tests for the Guide and chart fixes from the eighth two-agent trial
//! (ranked items 22, 23 and 35): the chart in the field's own shape with a
//! key and a compass, the Guide's roster with costs, keys and counters, a
//! TECH page for the reader's own faction, and a line of flavour per unit.
#[cfg(test)]
mod tests {
    use crate::canvas::Canvas;
    use crate::game::{Action, Game, Screen};
    use crate::minimap_chart::{ChartRect, MARGIN};
    use bw_core::{Faction, Kind, Pos};
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

    /// Save the frame, and a 3x crop of `region` when one is given, when
    /// `BW_GUIDE_CHART_FRAMES` names a folder, for review.
    fn keep_frame(canvas: &Canvas, name: &str, region: Option<(i32, i32, i32, i32)>) {
        let Ok(dir) = std::env::var("BW_GUIDE_CHART_FRAMES") else {
            return;
        };
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("frame folder");
        canvas
            .save(&dir.join(format!("{name}.png")))
            .expect("frame");
        if let Some((x, y, w, h)) = region {
            let mut crop = Canvas::new(w as u32 * 3, h as u32 * 3);
            for cy in 0..h {
                for cx in 0..w {
                    if let Some(c) = canvas.get(x + cx, y + cy) {
                        crop.rect(cx * 3, cy * 3, 3, 3, c);
                    }
                }
            }
            crop.save(&dir.join(format!("{name}-3x.png")))
                .expect("crop");
        }
    }

    #[test]
    fn the_chart_is_the_map_in_its_field_shape() {
        let mut g = game();
        let chart = g.chart();
        let bounds = g.minimap_bounds();
        assert_eq!(chart.w, 2 * chart.h, "a two-to-one diamond");
        assert!(chart.w <= bounds.w && chart.h <= bounds.h);
        let (mw, mh) = (i32::from(g.world.map.width), i32::from(g.world.map.height));
        // The corners of the map are the diamond's points: north-west at the
        // top, south-east at the bottom, as the camera shows them.
        let top = chart.plot(&g.world, Pos::raw(0, 0));
        let bottom = chart.plot(&g.world, Pos::raw(mw * 256, mh * 256));
        let left = chart.plot(&g.world, Pos::raw(0, mh * 256));
        let right = chart.plot(&g.world, Pos::raw(mw * 256, 0));
        assert_eq!(top, (chart.x + chart.w / 2, chart.y));
        assert_eq!(bottom, (chart.x + chart.w / 2, chart.y + chart.h));
        assert_eq!(left, (chart.x, chart.y + chart.h / 2));
        assert_eq!(right, (chart.x + chart.w, chart.y + chart.h / 2));
        // A chart pixel and the map point under it agree both ways.
        for cell in [(10, 20), (64, 64), (100, 30), (40, 110)] {
            let (x, y) = chart.plot(&g.world, Pos::cell(cell.0, cell.1));
            let back = chart.cell_at(&g.world, x, y).expect("on the map");
            assert!(
                (back.0 - cell.0).abs() <= 1 && (back.1 - cell.1).abs() <= 1,
                "{cell:?} came back as {back:?}"
            );
        }
        // The margin is off the map; a press there goes to the nearest edge.
        assert_eq!(chart.cell_at(&g.world, chart.x + 1, chart.y + 1), None);
        let edge = g.minimap_pos(chart.x + 1, chart.y + 1).cell_xy();
        assert!(edge.0 == 0 || edge.1 == 0, "{edge:?}");
        g.render();
        keep_frame(
            &g.canvas,
            "chart",
            Some((bounds.x - 4, bounds.y - 4, bounds.w + 8, bounds.h + 8)),
        );
        // The small-window chart takes the same shape.
        g.resize_view(640, 360);
        g.render();
        let small = g.chart();
        assert_eq!(small.w, 2 * small.h);
        keep_frame(&g.canvas, "chart-small", Some((9, 297, 118, 57)));
    }

    #[test]
    fn the_camera_footprint_on_the_chart_is_a_plain_rectangle() {
        let g = game();
        let chart = g.chart();
        let view = g.world_view();
        let corners = [
            (0, view.top),
            (view.width - 1, view.top),
            (view.width - 1, view.bottom - 1),
            (0, view.bottom - 1),
        ]
        .map(|(x, y)| chart.plot(&g.world, g.unproject(x, y)));
        // Top edge level, sides upright: within a pixel of rounding.
        assert!((corners[0].1 - corners[1].1).abs() <= 1, "{corners:?}");
        assert!((corners[2].1 - corners[3].1).abs() <= 1, "{corners:?}");
        assert!((corners[0].0 - corners[3].0).abs() <= 1, "{corners:?}");
        assert!((corners[1].0 - corners[2].0).abs() <= 1, "{corners:?}");
        // And as wide against tall as the view itself.
        let (fw, fh) = (corners[1].0 - corners[0].0, corners[3].1 - corners[0].1);
        let (vw, vh) = (view.width, view.bottom - view.top);
        assert!(
            (fw * vh - fh * vw).abs() <= 2 * vw.max(vh),
            "footprint {fw}x{fh} for a {vw}x{vh} view"
        );
    }

    #[test]
    fn the_chart_margin_carries_a_key_and_a_compass() {
        let mut g = game();
        g.render();
        let bounds = g.minimap_bounds();
        let chart = g.chart();
        let s = g.ui_scale();
        // Aged paper fills the corners the diamond leaves free.
        assert_eq!(
            g.canvas.get(bounds.x + bounds.w / 4, bounds.y + 12 * s),
            Some(MARGIN)
        );
        // Each key word and the compass's N are inked in the four corners.
        let ink = |x0: i32, y0: i32, x1: i32, y1: i32| {
            (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .filter(|&(x, y)| g.canvas.get(x, y) == Some(crate::minimap_chart::INK))
                .filter(|&(x, y)| chart.cell_at(&g.world, x, y).is_none())
                .count()
        };
        let (l, t, r, b) = (bounds.x, bounds.y, bounds.x + bounds.w, bounds.y + bounds.h);
        let (hw, hh) = (bounds.w / 2, bounds.h / 2);
        assert!(ink(l, t, l + hw, t + hh) > 30, "WELL key, top left");
        assert!(ink(l, t + hh, l + hw, b) > 30, "WRECK key, bottom left");
        assert!(ink(l + hw, t + hh, r, b) > 30, "MOUTH key, bottom right");
        assert!(ink(l + hw, t, r, t + hh) > 30, "compass, top right");
    }

    #[test]
    fn the_axis_chart_is_unchanged_for_the_observer_map() {
        let g = game();
        let r = ChartRect::axis(0, 0, 256, 256);
        assert_eq!(r.plot(&g.world, Pos::cell(64, 32)), (128, 64));
        assert_eq!(r.cell_at(&g.world, 128, 64), Some((64, 32)));
    }

    fn guide(g: &mut Game, page: usize) {
        g.open_screen(Screen::Help);
        g.action(Action::GuidePage(page));
        g.render();
    }

    /// Every glyph run of `text` in the small face, as `Canvas::text` draws it.
    fn drawn(g: &Game, text: &str, color: crate::canvas::Color) -> bool {
        let mut probe = Canvas::new(g.ui_canvas.width(), g.ui_canvas.height());
        probe.text(text, 0, 0, color);
        let w = text.len() as i32 * 6;
        // Find the glyph pattern anywhere on the menu canvas.
        let (cw, ch) = (g.ui_canvas.width() as i32, g.ui_canvas.height() as i32);
        let lit: Vec<(i32, i32)> = (0..7)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| probe.get(x, y) == Some(color))
            .collect();
        (0..ch - 7).any(|y0| {
            (0..cw - w).any(|x0| {
                lit.iter()
                    .all(|&(x, y)| g.ui_canvas.get(x0 + x, y0 + y) == Some(color))
            })
        })
    }

    #[test]
    fn arrows_on_the_tabs_turn_the_guide_pages() {
        let mut g = game();
        guide(&mut g, 2);
        g.key("ArrowRight", false, false);
        assert_eq!(g.ux.help_page, 3);
        g.key("ArrowLeft", false, false);
        g.key("ArrowLeft", false, false);
        assert_eq!(g.ux.help_page, 1);
        // Focus on a roster card moves between cards instead.
        guide(&mut g, crate::menus::ROSTER_PAGE);
        g.ux.focused = Some(Action::RosterUnit(Kind::Hook));
        g.ux.keyboard_navigation = true;
        g.key("ArrowRight", false, false);
        assert_eq!(g.ux.help_page, crate::menus::ROSTER_PAGE);
        assert!(matches!(g.ux.focused, Some(Action::RosterUnit(k)) if k != Kind::Hook));
    }

    #[test]
    fn the_roster_lists_cost_key_counters_and_flavour_for_every_machine() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut g = game();
            g.ux.roster_faction = Some(faction);
            guide(&mut g, crate::menus::ROSTER_PAGE);
            let machines = crate::menus::roster_machines(faction);
            assert_eq!(machines.len(), 8);
            for kind in &machines {
                assert!(!crate::menus::roster_train(faction, *kind).is_empty());
            }
            // Spot checks on the drawn page, one per column.
            let (worker, loom_or_bulwark) = if faction == Faction::Union {
                (Kind::Hook, Kind::Bulwark)
            } else {
                (Kind::Wick, Kind::Loom)
            };
            assert!(drawn(&g, worker.name(), crate::canvas::WHITE));
            assert!(drawn(&g, "Q AT HQ", crate::canvas::MUTED));
            assert!(drawn(&g, "R AT DRYDOCK", crate::canvas::MUTED));
            // A card's counters show under the grid once it is chosen.
            g.action(Action::RosterUnit(loom_or_bulwark));
            g.render();
            let (_, weak) = crate::ux::matchups(loom_or_bulwark);
            assert!(drawn(&g, &format!("LOSES TO {weak}"), crate::canvas::RED));
            keep_frame(
                &g.ui_canvas,
                &format!("guide-roster-{faction:?}").to_lowercase(),
                None,
            );
        }
        let mut g = game();
        guide(&mut g, crate::menus::ROSTER_PAGE);
        g.action(Action::RosterBuildings);
        g.render();
        assert!(drawn(&g, "DEFENSE NEST", crate::canvas::WHITE));
        g.action(Action::RosterUnit(Kind::Drydock));
        g.render();
        assert!(drawn(
            &g,
            crate::ux::flavour(Kind::Drydock),
            [196, 170, 128, 255]
        ));
        keep_frame(&g.ui_canvas, "guide-roster-buildings", None);
        g.action(Action::RosterFaction(Faction::Assembly));
        assert!(!g.ux.roster_buildings);
    }

    #[test]
    fn every_unit_has_a_flavour_line_that_fits_and_closes_its_tooltip() {
        for kind in crate::menus::ROSTER_KINDS {
            let line = crate::ux::flavour(kind);
            assert!(!line.is_empty() && line.len() <= 43, "{kind:?}: {line}");
            let tip = crate::ux::kind_tooltip(kind, "Q.");
            assert_eq!(tip.lines().last(), Some(line), "{kind:?}");
        }
    }

    #[test]
    fn the_tech_page_names_the_readers_own_machines() {
        let (_, union) = crate::menus::guide_page(6, Faction::Union);
        let (_, assembly) = crate::menus::guide_page(6, Faction::Assembly);
        assert!(union.iter().any(|(_, a)| a.contains("CAISSON")));
        assert!(!assembly.iter().any(|(_, a)| a.contains("CAISSON")));
        assert!(assembly.iter().any(|(_, a)| a.contains("LOOM")));
        let mut g = game();
        g.faction = Faction::Assembly;
        guide(&mut g, 6);
        keep_frame(&g.ui_canvas, "guide-tech-assembly", None);
    }

    #[test]
    fn the_guide_opens_on_basics_and_a_quick_click_lands() {
        let mut g = game();
        g.action(Action::Help);
        g.action(Action::GuidePage(6));
        g.action(Action::Help);
        assert_eq!(g.screen, Screen::Match);
        // Reopened: the first page, not the one left open.
        g.action(Action::Help);
        assert_eq!(g.ux.help_page, 0);
        // A click on the ROSTER tab before any frame is drawn still lands.
        assert!(g.buttons.is_empty());
        let s = g.ui_scale();
        let x = (g.canvas.width() as i32 - 640 * s) / 2;
        let y = (g.canvas.height() as i32 - 360 * s) / 2;
        let tab = 24 + crate::menus::ROSTER_PAGE as i32 * 74 + 36;
        g.left_down(x + tab * s, y + 58 * s);
        assert_eq!(g.ux.help_page, crate::menus::ROSTER_PAGE);
    }
}
