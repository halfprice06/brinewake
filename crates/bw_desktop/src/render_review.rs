//! Reproducible CPU-compositor measurements. This excludes GPU presentation.
use crate::game::Game;
use bw_core::{Faction, Pos};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for faction in [Faction::Union, Faction::Assembly] {
        let mut game =
            Game::new_with_data_dir(base.clone(), dir.join(format!("{faction:?}-profile")));
        game.faction = faction;
        crate::depth_review::practice(&mut game)?;
        for (name, pos) in [
            ("tactical", Pos::cell(33, 61)),
            ("north-crossing", Pos::cell(64, 49)),
            ("south-crossing", Pos::cell(64, 79)),
        ] {
            game.camera.center(pos);
            for _ in 0..10 {
                game.render();
            }
            let hash = game.world.state_hash();
            let mut samples = Vec::with_capacity(120);
            for _ in 0..120 {
                let begin = Instant::now();
                game.render();
                samples.push(begin.elapsed().as_secs_f64() * 1000.0);
            }
            if game.world.state_hash() != hash {
                return Err("Renderer mutated simulation".into());
            }
            samples.sort_by(f64::total_cmp);
            results.push(serde_json::json!({"faction":format!("{faction:?}"),"scene":name,"entities":game.world.entities.len(),"samples":samples.len(),"p50_ms":samples[60],"p95_ms":samples[114],"p99_ms":samples[118],"world_hash_unchanged":true}));
        }
    }
    let report = serde_json::json!({"scope":"CPU 640x360 compositor only; prepared small tactical/terrain fields; no GPU or minimum-hardware claim","results":results});
    std::fs::write(
        dir.join("render-times.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("{report}");
    Ok(())
}
