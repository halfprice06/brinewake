//! Controlled ordinary-start journeys; distinct from native or human play.
use crate::game::{Action, Game, Screen};
use bw_core::{Faction, Kind, Pos};
use std::path::{Path, PathBuf};

pub fn prepare(g: &mut Game) -> Result<(), String> {
    g.begin_practice();
    g.ux.guidance_visible = false;
    g.ux.tactics_fixture = true;
    g.ux.tutorial = None;
    for n in 0..2 {
        g.action(Action::FocusWorker);
        let site = if n == 0 {
            Pos::cell(20, 63)
        } else {
            (56..70)
                .flat_map(|y| (19..30).map(move |x| Pos::cell(x, y)))
                .find(|p| g.placement_reason(Kind::Works, *p).is_none())
                .ok_or("No second Works site")?
        };
        for _ in 0..3000 {
            if g.action_reason(&Action::Build(Kind::Works)).is_none() {
                break;
            }
            g.tick();
        }
        g.issue(bw_sim::Command::Build {
            worker: g.selected[0],
            kind: Kind::Works,
            pos: site,
            queued: false,
        });
        for _ in 0..3000 {
            g.tick();
            if g.works_ids().len() == n + 1 {
                break;
            }
        }
        if g.works_ids().len() != n + 1 {
            return Err("Works did not finish".into());
        }
    }
    for _ in 0..6000 {
        if g.world.players[0].salvage >= 600 {
            break;
        }
        g.tick();
    }
    g.key("F4", false, false);
    g.camera.center(Pos::cell(21, 63));
    Ok(())
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut g = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-profile")));
        g.faction = faction;
        prepare(&mut g)?;
        g.resize_view(1920, 1080);
        g.zoom = crate::zoom::Zoom::Wide;
        g.render();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Train(faction.army()[0]))
            .ok_or("Missing training button")?
            .clone();
        let from = g.world.command_log.len();
        g.ux.pointer_shift = true;
        g.left_down(b.x + b.w / 2, b.y + b.h / 2);
        g.ux.pointer_shift = false;
        g.tick();
        let batches = &g.world.command_log[from..];
        if batches.len() != 5 || batches.iter().any(|r| r.applied != Some(true)) {
            return Err(format!("{name}: batch mismatch: {batches:?}"));
        }
        let lengths: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| g.selected.contains(&e.id))
            .map(|e| e.queue.len())
            .collect();
        if lengths.len() != 2 || lengths.iter().max().unwrap() - lengths.iter().min().unwrap() > 1 {
            return Err("Batch not distributed".into());
        }
        g.cursor = (-999, -999);
        g.screenshot(&dir.join(format!("{name}-batch.png")))?;
        for _ in 0..900 {
            g.tick();
            if g.ux
                .alerts
                .entries
                .iter()
                .any(|a| matches!(a.kind, crate::field_alerts::AlertKind::Ready(_)))
            {
                break;
            }
        }
        if !g
            .ux
            .alerts
            .entries
            .iter()
            .any(|a| matches!(a.kind, crate::field_alerts::AlertKind::Ready(_)))
        {
            return Err("No production alert".into());
        }
        let hash = g.world.state_hash();
        let selected = g.selected.clone();
        g.key("F3", false, false);
        if hash != g.world.state_hash() || selected != g.selected {
            return Err("Recall changed field or selection".into());
        }
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.cursor = (-999, -999);
            g.screenshot(&dir.join(format!("{name}-alerts-{w}.png")))?;
        }
        let replay = dir.join(format!("{name}.replay.json"));
        g.world.export_replay(&replay)?;
        if bw_sim::World::replay(&replay)?.state_hash() != hash {
            return Err("Replay mismatch".into());
        }
        if !g.save_match() {
            return Err("Save failed".into());
        }
        g.load_match();
        if g.world.state_hash() != hash || !g.ux.alerts.entries.is_empty() {
            return Err("Load mismatch / stale alerts".into());
        }
        g.action(Action::Settings);
        for speed in crate::tempo::GameSpeed::all() {
            g.ux.preferences.game_speed = speed;
            g.screenshot(&dir.join(format!("{name}-settings-{}.png", speed.label())))?;
        }
        results.push(serde_json::json!({"faction":name,"ticks":g.world.tick,"hash":hash,"batch":lengths,"replay":true,"save_load":true}));
        // A separate explicitly prepared combat fixture tests actual Damage events.
        crate::depth_review::practice(&mut g)?;
        g.screen = Screen::Match;
        g.key("F2", false, false);
        g.issue(bw_sim::Command::AttackMove {
            units: g.selected.clone(),
            target: Pos::cell(37, 61),
            queued: false,
        });
        for _ in 0..600 {
            g.tick();
            if g.ux
                .alerts
                .entries
                .iter()
                .any(|a| a.kind == crate::field_alerts::AlertKind::Attack)
            {
                break;
            }
        }
        if !g
            .ux
            .alerts
            .entries
            .iter()
            .any(|a| a.kind == crate::field_alerts::AlertKind::Attack)
        {
            return Err("No actual combat alert".into());
        }
        g.key("F3", false, false);
        g.cursor = (-999, -999);
        g.screenshot(&dir.join(format!("{name}-combat.png")))?;
    }
    std::fs::write(
        dir.join("journeys.json"),
        serde_json::to_vec_pretty(&results).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
