//! Reproducible world-zoom and building-movement review.
use crate::game::Game;
use crate::zoom::Zoom;
use bw_core::{Faction, Pos};
use std::path::{Path, PathBuf};

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut game = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-session")));
        game.resize_view(1920, 1080);
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        game.message.clear();
        for zoom in [Zoom::Detail, Zoom::Wide, Zoom::Overview] {
            game.set_zoom(zoom, game.world_view().center());
            game.screenshot(&dir.join(format!("{name}-base-{}.png", zoom.percent())))?;
        }
        crate::artistry_review::practice(&mut game)?;
        game.message.clear();
        game.camera.center(Pos::cell(62, 64));
        let hash = game.world.state_hash();
        for zoom in [Zoom::Detail, Zoom::Wide, Zoom::Overview] {
            game.set_zoom(zoom, game.world_view().center());
            game.screenshot(&dir.join(format!("{name}-basin-{}.png", zoom.percent())))?;
            let mut times = Vec::new();
            for _ in 0..40 {
                let t = std::time::Instant::now();
                game.render();
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            std::fs::write(dir.join(format!("{name}-perf-{}.json",zoom.percent())),serde_json::to_vec_pretty(&serde_json::json!({"zoom":zoom.percent(),"p95_ms":times[38],"p99_ms":times[39],"samples":40,"scope":"CPU compositor only, controlled field, local hardware"})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        }
        if game.world.state_hash() != hash {
            return Err("Zoom changed simulation".into());
        }
        walk_around(base.clone(), dir, faction, name)?;
    }
    Ok(())
}

/// Continuous-zoom review: stills at whole and in-between scales, the frames
/// of an eased wheel zoom at 60 frames a second, and compositor timings.
pub fn export_zoom(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut game = Game::new_with_data_dir(base, dir.join("session"));
    game.resize_view(1920, 1080);
    game.faction = Faction::Union;
    game.start();
    game.world.ai_enabled = false;
    crate::artistry_review::practice(&mut game)?;
    game.message.clear();
    game.camera.center(Pos::cell(62, 64));
    let hash = game.world.state_hash();
    let center = game.world_view().center();
    let mut timings = Vec::new();
    for scale in [1.0, 1.26, 1.59, 2.0, 2.52, 3.0] {
        game.set_zoom(Zoom::new(scale), center);
        game.screenshot(&dir.join(format!("still-{}.png", Zoom::new(scale).percent())))?;
        let mut times = Vec::new();
        for _ in 0..40 {
            let t = std::time::Instant::now();
            game.render();
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        timings.push(serde_json::json!({"scale": scale, "p95_ms": times[38], "p99_ms": times[39]}));
    }
    std::fs::write(
        dir.join("perf.json"),
        serde_json::to_vec_pretty(&timings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // Wheel in toward a point off centre, one click every six frames, then
    // back out, as a hand on a mouse wheel would.
    game.set_zoom(Zoom::Overview, center);
    let anchor = (center.0 + 300, center.1 - 120);
    let mut frame = 0;
    for (clicks, notch) in [(5, 1.0), (5, -1.0)] {
        for tick in 0..(clicks * 6 + 30) {
            if tick % 6 == 0 && tick / 6 < clicks {
                game.wheel_zoom(notch, anchor);
            }
            if let Some(goal) = game.zoom_goal.as_mut() {
                let d = std::time::Duration::from_nanos(16_666_667);
                goal.last_frame -= d;
                goal.since -= d;
            }
            game.advance_zoom();
            game.render();
            game.canvas
                .save(&dir.join(format!("wheel-{frame:04}.png")))?;
            frame += 1;
        }
    }
    if game.world.state_hash() != hash {
        return Err("Zoom changed simulation".into());
    }
    Ok(())
}

fn walk_around(base: PathBuf, dir: &Path, faction: Faction, name: &str) -> Result<(), String> {
    use bw_core::{FP, Kind};
    let mut g = Game::new_with_data_dir(base, dir.join(format!("{name}-walk-session")));
    g.resize_view(1280, 720);
    g.faction = faction;
    g.start();
    g.world.ai_enabled = false;
    g.message.clear();
    for e in &mut g.world.entities {
        e.order = bw_sim::Order::Idle;
        e.path.clear();
        e.path_index = 0;
        e.path_target = None;
        e.waypoints.clear();
    }
    let hq = g
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .unwrap();
    hq.pos = Pos::cell(30, 60);
    let hq_id = hq.id;
    let unit = g
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 0 && e.kind.is_worker())
        .unwrap();
    unit.pos = Pos::cell(28, 58);
    let id = unit.id;
    let bounds = crate::occlusion::footprint_bounds(
        g.world.entities.iter().find(|e| e.id == hq_id).unwrap(),
    )
    .unwrap();
    g.camera.center(Pos::cell(32, 61));
    g.zoom = Zoom::Wide;
    g.selected = vec![id];
    g.world.reset_fixture_origin()?;
    for (i, p) in [(35, 58), (35, 65), (28, 65), (28, 58)]
        .into_iter()
        .enumerate()
    {
        g.issue(bw_sim::Command::Move {
            units: vec![id],
            target: Pos::cell(p.0, p.1),
            queued: i != 0,
        });
    }
    let mut positions = Vec::new();
    let mut frames = Vec::new();
    for _ in 0..1600 {
        g.tick();
        let e = g.world.entities.iter().find(|e| e.id == id).unwrap();
        if bounds.contains(e.pos) {
            return Err("Worker entered building footprint".into());
        }
        positions.push(serde_json::json!({"tick":g.world.tick,"x":e.pos.x,"y":e.pos.y}));
        let arrived = g.world.tick > 30
            && e.pos.distance_sq(Pos::cell(28, 58)) < i64::from(FP / 8).pow(2)
            && e.waypoints.is_empty();
        if g.world.tick.is_multiple_of(4) && frames.len() < 200 {
            let file = format!("{name}-walk-{:03}.png", frames.len());
            g.screenshot(&dir.join(&file))?;
            frames.push(file);
        }
        if arrived {
            break;
        }
    }
    let unit = g.world.entities.iter().find(|e| e.id == id).unwrap();
    if unit.pos.distance_sq(Pos::cell(28, 58)) >= i64::from(FP / 8).pow(2)
        || !unit.waypoints.is_empty()
    {
        return Err("Building perimeter journey did not finish".into());
    }
    let max_x = positions
        .iter()
        .filter_map(|p| p["x"].as_i64())
        .max()
        .unwrap_or(0);
    let max_y = positions
        .iter()
        .filter_map(|p| p["y"].as_i64())
        .max()
        .unwrap_or(0);
    if max_x < i64::from(Pos::cell(35, 58).x - FP / 8)
        || max_y < i64::from(Pos::cell(35, 65).y - FP / 8)
        || g.world
            .command_log
            .iter()
            .filter(|c| c.applied == Some(true))
            .count()
            < 4
    {
        return Err("Perimeter journey did not traverse every requested side".into());
    }
    g.render();
    let corners = [
        Pos::raw(bounds.left, bounds.top),
        Pos::raw(bounds.right, bounds.top),
        Pos::raw(bounds.right, bounds.bottom),
        Pos::raw(bounds.left, bounds.bottom),
    ];
    for i in 0..4 {
        let (a, b) = g.project(corners[i]);
        let (c, d) = g.project(corners[(i + 1) % 4]);
        g.canvas.line(a, b, c, d, crate::canvas::JADE);
    }
    g.canvas.text(
        "CONTROLLED WALK / ACTUAL BLOCKED FOOTPRINT",
        8,
        64,
        crate::canvas::GOLD,
    );
    g.canvas.save(&dir.join(format!("{name}-footprint.png")))?;
    let replay = dir.join(format!("{name}-walk.replay.json"));
    g.world.export_replay(&replay)?;
    if bw_sim::World::replay(&replay)?.state_hash() != g.world.state_hash() {
        return Err("Walking replay diverged".into());
    }
    std::fs::write(dir.join(format!("{name}-walk.json")),serde_json::to_vec_pretty(&serde_json::json!({"evidence":"prepared scene, normal queued movement commands, fixed-point checks every tick, controlled renderer frames","positions":positions,"frames":frames,"hash":g.world.state_hash()})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Action;
    fn game(name: &str) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir =
            std::env::temp_dir().join(format!("brinewake-camera-{name}-{}", std::process::id()));
        let mut g = Game::new_with_data_dir(base, dir);
        g.start();
        g.world.ai_enabled = false;
        g.message.clear();
        g
    }
    #[test]
    fn zoom_anchor_world_point_is_preserved() {
        let mut g = game("anchor");
        for anchor in [(320, 156), (400, 182), (41, 131)] {
            for z in [Zoom::Overview, Zoom::Wide, Zoom::Detail] {
                let before = g.unproject(anchor.0, anchor.1);
                g.set_zoom(z, anchor);
                assert_eq!(g.unproject(anchor.0, anchor.1), before);
            }
        }
    }
    #[test]
    fn selection_and_movement_use_visible_zoomed_pixels() {
        let mut g = game("input");
        let worker = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap();
        worker.pos = Pos::cell(30, 61);
        worker.order = bw_sim::Order::Idle;
        worker.path.clear();
        worker.path_index = 0;
        let id = worker.id;
        g.camera.center(Pos::cell(30, 61));
        for z in [Zoom::Detail, Zoom::Wide, Zoom::Overview] {
            g.set_zoom(z, (320, 156));
            g.selected.clear();
            g.render();
            let (cx, cy) = g.project(Pos::cell(30, 61));
            let hit = (-40..8)
                .flat_map(|dy| (-24..24).map(move |dx| (cx + dx, cy + dy)))
                .find(|&(x, y)| g.unit_at(x, y) == Some(id))
                .expect("visible worker texel");
            g.left_down(hit.0, hit.1);
            g.left_up(hit.0, hit.1, false);
            assert!(g.selected.contains(&id));
            let target = Pos::cell(32, 63);
            let p = g.project(target);
            g.right_click(p.0, p.1, false);
            assert!(
                matches!(&g.world.command_log.last().unwrap().command,bw_sim::Command::Move{target:t,..}if t.cell_xy()==target.cell_xy())
            );
        }
    }
    #[test]
    fn zoom_render_preserves_ui_scale_and_world_hash() {
        let mut g = game("pixels");
        g.selected.clear();
        let before = g.world.state_hash();
        g.zoom = Zoom::Overview;
        g.render();
        let pixels = g.canvas.pixels.clone();
        for z in [Zoom::Wide, Zoom::Overview] {
            g.set_zoom(z, (320, 156));
            g.render();
            assert_eq!(&g.canvas.pixels[..640 * 24 * 4], &pixels[..640 * 24 * 4]);
            for y in 288..360 {
                let range = (y * 640 + 132) * 4..(y * 640 + 640) * 4;
                assert_eq!(&g.canvas.pixels[range.clone()], &pixels[range]);
            }
            assert_eq!(g.world.state_hash(), before);
            assert_eq!((g.canvas.width(), g.canvas.height()), (640, 360));
            assert_eq!(
                (g.scene_canvas.width(), g.scene_canvas.height()),
                g.world_view().size()
            );
        }
    }
    #[test]
    fn zoom_buttons_keys_and_preference_agree() {
        let mut g = game("controls");
        assert_eq!(crate::ux::Preferences::default().world_zoom, 2.0);
        g.zoom = Zoom::Detail;
        g.action(Action::ZoomOut);
        // The step eases in over the next frames.
        assert_eq!(g.zoom, Zoom::Detail);
        g.settle_zoom();
        assert_eq!(g.zoom, Zoom::Wide);
        g.key("-", false, false);
        g.settle_zoom();
        assert_eq!(g.zoom, Zoom::Overview);
        g.key("+", false, false);
        g.settle_zoom();
        assert_eq!(g.zoom, Zoom::Wide);
        let prefs = crate::ux::UxState::load(&g.data_dir);
        assert_eq!(prefs.preferences.world_zoom, 2.0);
        g.render();
        assert!(g.buttons.iter().any(|b| b.action == Action::ZoomOut));
    }
    #[test]
    fn wheel_zoom_is_continuous_and_keeps_the_pointed_world_still() {
        let mut g = game("wheel");
        g.resize_view(1920, 1080);
        g.zoom = Zoom::Wide;
        let anchor = (700, 500);
        let exact = |g: &Game| {
            let v = g.world_view();
            let (sx, sy) = v.source_exact(anchor);
            let c = v.scene_camera(g.camera);
            let e = v.source_exact((0, v.top));
            // World-texel coordinate under the anchor, sub-texel included.
            (f64::from(c.x) + sx, f64::from(c.y) + sy, e)
        };
        let before = exact(&g);
        g.wheel_zoom(1.0, anchor);
        let mut seen = Vec::new();
        for _ in 0..40 {
            if let Some(goal) = g.zoom_goal.as_mut() {
                goal.last_frame -= std::time::Duration::from_millis(16);
            }
            g.advance_zoom();
            seen.push(g.zoom.scale());
            let now = exact(&g);
            assert!((now.0 - before.0).abs() < 0.5 && (now.1 - before.1).abs() < 0.5);
        }
        // It passed through scales between the steps, and one click is not
        // a whole step.
        assert!(seen.iter().any(|s| *s > 2.05 && *s < 2.45));
        g.settle_zoom();
        assert!((g.zoom.scale() - 2.0 * crate::zoom::WHEEL_RATIO).abs() < 1e-9);
        // Three clicks back out from there lands exactly on the next step.
        g.wheel_zoom(-3.0, anchor);
        g.settle_zoom();
        assert!((g.zoom.scale() - 2.0 / crate::zoom::WHEEL_RATIO.powi(2)).abs() < 1e-9);
        // A whole step is pixel-exact again after an in-between scale.
        g.set_zoom(Zoom::new(1.37), anchor);
        g.render();
        let texel = g.unproject(anchor.0, anchor.1);
        g.set_zoom(Zoom::Detail, anchor);
        assert_eq!(g.unproject(anchor.0, anchor.1), texel);
        let v = g.world_view();
        for (_, w) in v.taps(0, v.width, false, 4000) {
            assert_eq!(w, 0);
        }
        for (_, w) in v.taps(v.top, v.bottom, true, 4000) {
            assert_eq!(w, 0);
        }
    }
    #[test]
    fn panning_between_texels_moves_by_window_pixels() {
        let mut g = game("subpan");
        g.resize_view(1920, 1080);
        g.zoom = Zoom::Detail;
        g.render();
        let start = g.project(Pos::cell(30, 61));
        for step in 1..=6 {
            g.pan(1, 0);
            g.render();
            assert_eq!(g.project(Pos::cell(30, 61)).0, start.0 - step);
        }
    }
}
