//! Field marks captures: the words and marks drawn on the ground, at rest
//! and under the pointer, at 1x, 2x and 3x.
//!
//! `brinewake --field-marks-review <dir>` writes PNGs at 1280x720.
use crate::game::Game;
use crate::zoom::Zoom;
use bw_core::{Faction, Kind, Pos};
use std::path::{Path, PathBuf};

const ZOOMS: [(Zoom, &str); 3] = [
    (Zoom::Overview, "1x"),
    (Zoom::Wide, "2x"),
    (Zoom::Detail, "3x"),
];

fn shot(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    game.message.clear();
    game.screenshot(&dir.join(format!("{name}.png")))
}

/// A Union base at the west bank: two wells, the home wrecks, a scrap
/// heap, a stalled site and a queued one, and an enemy tower in sight.
fn base_scene(base: PathBuf, dir: &Path) -> Result<Game, String> {
    let mut game = Game::new_with_data_dir(base, dir.join("base-data"));
    game.faction = Faction::Union;
    game.start();
    game.world.ai_enabled = false;
    game.resize_view(1280, 720);
    game.ux.preferences.pause_unfocused = false;
    game.world.revealed = true;
    for (kind, cell) in [(Kind::Works, (24, 60)), (Kind::Tower, (25, 70))] {
        let id = game
            .world
            .spawn_for_tests(0, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) {
            e.build_remaining = bw_content::spec(kind).build_ticks / 2;
            e.hp = e.max_hp / 2;
        }
    }
    game.world
        .spawn_for_tests(1, Kind::Tower, Pos::cell(30, 64));
    let next = game
        .world
        .map
        .resources
        .iter()
        .map(|r| r.id)
        .max()
        .unwrap_or(0)
        + 1;
    for (i, (cell, amount)) in [((22, 56), 60u32), ((23, 57), 45)].into_iter().enumerate() {
        game.world.map.resources.push(bw_sim::Resource {
            id: next + i as u32,
            pos: Pos::cell(cell.0, cell.1),
            remaining: amount,
            kind: bw_sim::ResourceKind::Salvage,
            scrap: true,
        });
    }
    game.selected.clear();
    game.cursor = (-999, -999);
    Ok(game)
}

/// The mouths: the north lane held, an enemy gun at the south-west mouth,
/// the south lane deep.
fn mouth_scene(base: PathBuf, dir: &Path) -> Result<Game, String> {
    let mut game = Game::new_with_data_dir(base, dir.join("mouth-data"));
    game.faction = Faction::Union;
    game.start();
    game.world.ai_enabled = false;
    game.resize_view(1280, 720);
    game.ux.preferences.pause_unfocused = false;
    game.world.revealed = true;
    game.world.gate.owner = Some(0);
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::NORTH;
    for cell in [(49, 49), (78, 49), (48, 80)] {
        game.world
            .spawn_for_tests(0, Kind::Riveter, Pos::cell(cell.0, cell.1));
    }
    game.world
        .spawn_for_tests(1, Kind::Reedguard, Pos::cell(51, 78));
    game.world.lane_hold[0] = bw_content::TIDE_HOLD_TICKS / 3;
    game.selected.clear();
    game.cursor = (-999, -999);
    Ok(game)
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut game = base_scene(base.clone(), dir)?;
    for (zoom, label) in ZOOMS {
        game.zoom = zoom;
        game.camera.center(Pos::cell(22, 63));
        game.cursor = (-999, -999);
        shot(&mut game, dir, &format!("base-{label}"))?;
    }
    // Under the pointer: a well, a wreck, the stalled site, the enemy tower.
    game.zoom = Zoom::Wide;
    for (name, pos) in [
        ("well", game.world.map.wells[0]),
        ("wreck", Pos::cell(16, 57)),
        ("scrap", Pos::cell(22, 56)),
        ("site", Pos::cell(24, 60)),
        ("enemy", Pos::cell(30, 64)),
    ] {
        game.camera.center(Pos::cell(22, 63));
        let (x, y) = game.project(pos);
        game.cursor = (x, y - 4);
        shot(&mut game, dir, &format!("base-2x-hover-{name}"))?;
    }
    // The card keeps the interface's size at every zoom.
    for (zoom, label) in [(Zoom::Overview, "1x"), (Zoom::Detail, "3x")] {
        game.zoom = zoom;
        let well = game.world.map.wells[0];
        game.camera.center(well);
        let (x, y) = game.project(well);
        game.cursor = (x, y - 4);
        shot(&mut game, dir, &format!("base-{label}-hover-well"))?;
    }
    let mut game = mouth_scene(base, dir)?;
    for (zoom, label) in ZOOMS {
        game.zoom = zoom;
        game.camera.center(Pos::cell(56, 64));
        game.cursor = (-999, -999);
        shot(&mut game, dir, &format!("mouths-{label}"))?;
    }
    game.zoom = Zoom::Wide;
    for (name, pos) in [
        ("nw", bw_sim::CROSSING_MOUTHS[0][0]),
        ("sw", bw_sim::CROSSING_MOUTHS[1][0]),
        ("blocker", Pos::cell(51, 78)),
    ] {
        game.camera.center(pos);
        let (x, y) = game.project(Pos {
            x: pos.x + bw_core::FP * 2,
            y: pos.y,
        });
        game.cursor = if name == "blocker" {
            game.project(pos)
        } else {
            (x, y)
        };
        shot(&mut game, dir, &format!("mouths-2x-hover-{name}"))?;
    }
    Ok(())
}
