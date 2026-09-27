//! Root-authored real-command journeys plus controlled presentation fixtures.
use crate::game::{Action, Game, Screen};
use bw_core::{Faction, Pos};
use std::path::{Path, PathBuf};

fn click(g: &mut Game, action: Action, shift: bool) -> Result<(), String> {
    g.render();
    let b = g
        .buttons
        .iter()
        .find(|b| b.action == action && b.enabled)
        .ok_or_else(|| format!("Missing button {action:?}"))?
        .clone();
    g.ux.pointer_shift = shift;
    g.left_down(b.x + b.w / 2, b.y + b.h / 2);
    g.left_up(b.x + b.w / 2, b.y + b.h / 2, shift);
    g.ux.pointer_shift = false;
    Ok(())
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut results = vec![];
    for faction in [Faction::Union, Faction::Assembly] {
        let name = if faction == Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut g = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-profile")));
        g.faction = faction;
        crate::qol_v2_review::prepare(&mut g)?;
        g.resize_view(1920, 1080);
        g.zoom = crate::zoom::Zoom::Wide;
        let works = g.selected.clone();
        g.key("1", false, true);
        for kind in faction.army() {
            g.action(Action::Train(kind));
            g.tick();
        }
        for _ in 0..3000 {
            if faction.army().iter().all(|k| {
                g.world
                    .entities
                    .iter()
                    .any(|e| e.owner == 0 && e.kind == *k)
            }) {
                break;
            }
            g.tick();
        }
        if faction.army().iter().any(|k| {
            !g.world
                .entities
                .iter()
                .any(|e| e.owner == 0 && e.kind == *k)
        }) {
            return Err("Training did not complete".into());
        }
        g.key("F2", false, false);
        g.key("2", false, true);
        let army = g.selected.clone();
        let hash = g.world.state_hash();
        click(
            &mut g,
            Action::FilterSelection(faction.army()[0], false),
            false,
        )?;
        if g.selected.len() != 1 || g.world.state_hash() != hash {
            return Err("Type isolation failed".into());
        }
        click(&mut g, Action::RecallGroup(2), false)?;
        if g.selected != army {
            return Err("Group recall failed".into());
        }
        click(
            &mut g,
            Action::FilterSelection(faction.army()[0], false),
            true,
        )?;
        if g.selected.len() != 2 || g.world.state_hash() != hash {
            return Err("Type removal changed world".into());
        }
        click(&mut g, Action::RecallGroup(2), true)?;
        if g.selected != army || g.world.state_hash() != hash {
            return Err("Group center failed".into());
        }
        g.key("O", false, false);
        g.camera.center(Pos::cell(21, 63));
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.cursor = (-999, -999);
            g.message.clear();
            g.screenshot(&dir.join(format!("{name}-roster-{w}.png")))?;
        }
        click(&mut g, Action::RecallGroup(1), false)?;
        if g.selected != works {
            return Err("Production group recall failed".into());
        }
        // Build a real mixed queue at one producer, after waiting for income.
        g.selected = vec![works[0]];
        for _ in 0..4000 {
            if g.world.players[0].salvage >= 600 {
                break;
            }
            g.tick();
        }
        for kind in faction.army() {
            g.action(Action::Train(kind));
            g.tick();
        }
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.cursor = (-999, -999);
            g.message.clear();
            g.screenshot(&dir.join(format!("{name}-production-{w}.png")))?;
        }
        let hash = g.world.state_hash();
        let replay = dir.join(format!("{name}.replay.json"));
        g.world.export_replay(&replay)?;
        if bw_sim::World::replay(&replay)?.state_hash() != hash {
            return Err("Replay mismatch".into());
        }
        if !g.save_match() {
            return Err("Save failed".into());
        }
        g.load_match();
        if g.world.state_hash() != hash {
            return Err("Save/load mismatch: world hash changed".into());
        }
        if g.groups[1] != works || g.groups[2] != army {
            return Err(format!(
                "Save/load mismatch: groups {:?} / {:?} expected {:?} / {:?}",
                g.groups[1], g.groups[2], works, army
            ));
        }
        g.action(Action::Settings);
        g.screenshot(&dir.join(format!("{name}-settings.png")))?;
        results.push(serde_json::json!({"faction":name,"tick":g.world.tick,"hash":hash,"replay":true,"save_load_groups":true,"type_keep_remove":true,"group_click_center":true}));
        // Labelled engineering fixture stresses pagination and health-bar modes.
        crate::depth_review::practice(&mut g)?;
        g.screen = Screen::Match;
        g.selected = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0)
            .map(|e| e.id)
            .collect();
        g.resize_view(1280, 720);
        g.cursor = (-999, -999);
        g.message.clear();
        g.screenshot(&dir.join(format!("{name}-mixed-fixture.png")))?;
    }
    std::fs::write(
        dir.join("journeys.json"),
        serde_json::to_vec_pretty(&results).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
