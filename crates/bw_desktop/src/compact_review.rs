//! Compact captures: the Salter's causeway, the Heliostat's beam, GLINT (its
//! flash, ray and motes at 2X and 1X), glaze traces in the lane and
//! a boiling Pan, each with its command card. Controlled fixtures, not
//! playtest evidence.
//!
//! `brinewake --compact-review <dir>` writes PNGs at 1280x720.
use crate::game::{Action, Game};
use crate::zoom::Zoom;
use bw_core::{Faction, Kind, Pos};
use bw_sim::Order;
use std::path::{Path, PathBuf};

fn shot(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    game.screenshot(&dir.join(format!("{name}.png")))
}

fn scene(base: PathBuf, dir: &Path, name: &str) -> Result<Game, String> {
    let mut game = Game::new_with_data_dir(base, dir.join(format!("{name}-data")));
    game.faction = Faction::Compact;
    game.opponent = Some(Faction::Union);
    game.start();
    game.intro = None;
    game.world.ai_enabled = false;
    game.resize_view(1280, 720);
    game.ux.preferences.pause_unfocused = false;
    game.world.revealed = true;
    game.world.players[0].pressure = 150;
    game.zoom = Zoom::Wide;
    game.selected.clear();
    game.cursor = (-999, -999);
    Ok(game)
}

fn unit(game: &mut Game, owner: u8, kind: Kind, cell: (i32, i32), facing: u8) -> u32 {
    let id = game
        .world
        .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
    if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) {
        e.order = Order::Hold;
        e.facing = facing;
    }
    id
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

pub fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    // LAY across the flooded north lane from its west bank.
    let mut game = scene(base.clone(), dir, "lay")?;
    game.world.gate.tide = bw_sim::Tide::Flood;
    let salter = unit(&mut game, 0, Kind::Salter, (49, 49), 2);
    game.world
        .entities
        .iter_mut()
        .find(|e| e.id == salter)
        .ok_or("salter missing")?
        .order = Order::Idle;
    game.camera.center(Pos::cell(60, 50));
    game.selected = vec![salter];
    game.action(Action::Lay);
    let (x, y) = game.project(Pos::cell(70, 49));
    game.cursor = (x, y);
    shot(&mut game, dir, "lay-aim")?;
    game.left_down(x, y);
    game.left_up(x, y, false);
    game.cursor = (-999, -999);
    run(&mut game, 30 * 9 + 20);
    shot(&mut game, dir, "lay-working")?;
    run(&mut game, 30 * 20);
    shot(&mut game, dir, "lay-crossed")?;

    // A deployed Heliostat builds its beam on a Riveter.
    let mut game = scene(base.clone(), dir, "beam")?;
    let heliostat = unit(&mut game, 0, Kind::Heliostat, (30, 62), 2);
    unit(&mut game, 1, Kind::Riveter, (35, 62), 6);
    if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == heliostat) {
        e.deployed = true;
        e.order = Order::Idle;
    }
    game.camera.center(Pos::cell(32, 62));
    game.selected = vec![heliostat];
    // Six shots in, caught just after the seventh leaves the mirror.
    let mut shots = 0;
    for _ in 0..600 {
        game.tick();
        let fresh = game
            .effects
            .iter()
            .any(|effect| effect.beam.is_some() && effect.until == game.world.tick + 12 - 2);
        if fresh {
            shots += 1;
            if shots >= 7 {
                break;
            }
        }
    }
    if shots < 7 {
        return Err(format!("the Heliostat fired {shots} beams"));
    }
    shot(&mut game, dir, "beam-built")?;

    // GLINT down the south lane from the west bank.
    let mut game = scene(base.clone(), dir, "glint")?;
    let glinter = unit(&mut game, 0, Kind::Glinter, (46, 79), 2);
    game.world.revealed = false;
    game.camera.center(Pos::cell(55, 79));
    game.selected = vec![glinter];
    game.action(Action::Glint);
    let (x, y) = game.project(Pos::cell(60, 79));
    game.cursor = (x, y);
    shot(&mut game, dir, "glint-aim")?;
    game.left_down(x, y);
    game.left_up(x, y, false);
    game.cursor = (-999, -999);
    // The heliograph flash at the mirror, at 2X and 1X (art v23).
    run(&mut game, 2);
    shot(&mut game, dir, "glint-flash")?;
    game.zoom = Zoom::Overview;
    shot(&mut game, dir, "glint-flash-1x")?;
    game.zoom = Zoom::Wide;
    run(&mut game, 6);
    shot(&mut game, dir, "glint-lit")?;
    // Motes running out along the ray, the ring at its end.
    run(&mut game, 30);
    shot(&mut game, dir, "glint-run")?;
    game.zoom = Zoom::Overview;
    shot(&mut game, dir, "glint-run-1x")?;

    // Glaze fused into the dry south lane where a Heliostat's beam lands
    // on a Riveter crossing it (art v23 trace_glaze).
    let mut game = scene(base.clone(), dir, "glaze")?;
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(false);
    let heliostat = unit(&mut game, 0, Kind::Heliostat, (60, 79), 2);
    unit(&mut game, 1, Kind::Riveter, (65, 79), 6);
    if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == heliostat) {
        e.deployed = true;
        e.order = Order::Idle;
    }
    game.camera.center(Pos::cell(63, 79));
    let mut ran = 0;
    while game.ux.traces.traces.is_empty() && ran < 600 {
        game.tick();
        ran += 1;
    }
    if game.ux.traces.traces.is_empty() {
        return Err(format!(
            "no glaze trace after 600 ticks (target cell on {:?})",
            game.world.map.terrain(65, 79)
        ));
    }
    // Step the Riveter off its first mark so the glass shows beside it.
    if let Some(e) = game
        .world
        .entities
        .iter_mut()
        .find(|e| e.owner == 1 && e.kind == Kind::Riveter)
    {
        e.pos = Pos::cell(66, 81);
    }
    run(&mut game, 40);
    shot(&mut game, dir, "glaze")?;
    game.zoom = Zoom::Overview;
    shot(&mut game, dir, "glaze-1x")?;

    // Two Pans boiling beside the headquarters.
    let mut game = scene(base, dir, "pan")?;
    let hq = game
        .world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .map(|e| e.pos.cell_xy())
        .ok_or("headquarters missing")?;
    let pans = [
        unit(&mut game, 0, Kind::Pan, (hq.0 + 8, hq.1 + 8), 2),
        unit(&mut game, 0, Kind::Pan, (hq.0 + 10, hq.1 + 7), 2),
    ];
    game.world.players[0].pressure = 20;
    game.camera.center(Pos::cell(hq.0 + 7, hq.1 + 7));
    game.selected = pans.to_vec();
    game.action(Action::Deploy);
    run(&mut game, 60);
    shot(&mut game, dir, "pan-boiling")?;
    Ok(())
}
