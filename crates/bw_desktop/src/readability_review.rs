//! Battlefield readability captures: the spots the eighth trial named,
//! replayed from its recording, plus staged fixtures for the marks that a
//! recording cannot pose on demand.
//!
//! `brinewake --readability-review <dir> [--replay <file>]` writes PNGs at
//! 1280x720 (the interface at two) and 2X field zoom.
use crate::game::Game;
use crate::zoom::Zoom;
use bw_core::{Faction, Kind, Pos};
use std::path::{Path, PathBuf};

fn prepare(game: &mut Game) {
    game.resize_view(1280, 720);
    game.zoom = Zoom::Wide;
    game.ux.preferences.pause_unfocused = false;
}

fn shot(game: &mut Game, dir: &Path, name: &str) -> Result<(), String> {
    game.screenshot(&dir.join(format!("{name}.png")))
}

/// Step a recording to `tick`, keeping the seat's exploration up to date.
fn replay_to(game: &mut Game, player: &mut bw_sim::ReplayPlayer, tick: u64) -> Result<(), String> {
    while player.tick() < tick {
        if !player.step()? {
            break;
        }
        let mut world = player.world().clone();
        world.revealed = false;
        game.world = world;
        game.ux.chart_memory.observe(&game.world);
    }
    Ok(())
}

pub fn export(base: PathBuf, dir: &Path, replay: Option<&Path>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if let Some(path) = replay {
        let mut game = Game::new_with_data_dir(base.clone(), dir.join("replay-data"));
        let mut player = bw_sim::ReplayPlayer::open(path)?;
        game.faction = player.world().players[0].faction;
        game.start();
        game.world = player.world().clone();
        game.ux.chart_memory = Default::default();
        prepare(&mut game);
        game.message.clear();
        // 07:35, six plates over the station fight; 17:13, ten over the
        // Loom line at Union's north mouth.
        for (name, tick, at) in [
            ("replay-0735-station", 455 * 30, None),
            ("replay-1134-far-bank", 694 * 30, Some(Pos::cell(84, 60))),
            (
                "replay-1713-north-mouth",
                1033 * 30,
                Some(Pos::cell(50, 49)),
            ),
        ] {
            replay_to(&mut game, &mut player, tick)?;
            let at = at.unwrap_or(game.world.map.gate_pos);
            for (suffix, revealed) in [("seat", false), ("observer", true)] {
                game.world.revealed = revealed;
                game.camera.center(at);
                game.selected.clear();
                game.message.clear();
                shot(&mut game, dir, &format!("{name}-{suffix}"))?;
            }
            game.world.revealed = false;
        }
    }
    staged(base.clone(), dir)?;
    staged_tide(base, dir)
}

/// The south lane deep and walled, and a Loom shell in the air over it.
fn staged_tide(base: PathBuf, dir: &Path) -> Result<(), String> {
    let mut game = Game::new_with_data_dir(base, dir.join("tide-data"));
    game.faction = Faction::Union;
    game.start();
    game.world.ai_enabled = false;
    prepare(&mut game);
    game.world.gate.owner = Some(0);
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
    // An enemy Loom on our bank lobs a shell at a Riveter.
    let loom = game.world.spawn_for_tests(1, Kind::Loom, Pos::cell(43, 75));
    if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == loom) {
        e.deployed = true;
    }
    // A shell's flight starts before its impact tick; give it room.
    game.world.tick = 300;
    for (i, cell) in [(46, 80), (47, 78), (45, 82)].into_iter().enumerate() {
        let id = game
            .world
            .spawn_for_tests(0, Kind::Riveter, Pos::cell(cell.0, cell.1));
        let _ = (i, id);
    }
    let tick = game.world.tick;
    for (frame, left) in [(0, 12u64), (1, 7), (2, 2)] {
        game.world.artillery.clear();
        game.world.artillery.push(bw_sim::ArtilleryShot {
            owner: 1,
            source: loom,
            from: Pos::cell(43, 75),
            target: Pos::cell(47, 78),
            impact_tick: tick + left,
        });
        game.camera.center(Pos::cell(49, 78));
        game.selected.clear();
        game.message.clear();
        shot(&mut game, dir, &format!("staged-deep-lane-shell-{frame}"))?;
    }
    game.world.artillery.clear();
    // The lock turning to dry the south shaft, at close zoom.
    game.zoom = Zoom::Detail;
    // The gate stands tall: centre a little north of its foot.
    game.camera.center(Pos::cell(61, 61));
    game.world.gate.dry_arm = bw_sim::Arm::from_north(false);
    for frame in [0u64, 2, 5, 8, 11] {
        game.gate_switch = Some((
            tick - frame * crate::presentation::GATE_SWITCH_FRAME_TICKS,
            false,
        ));
        shot(&mut game, dir, &format!("staged-lock-switch-{frame:02}"))?;
    }
    game.gate_switch = None;
    game.zoom = Zoom::Overview;
    game.camera.center(Pos::cell(64, 64));
    shot(&mut game, dir, "staged-deep-lane-1x")
}

/// Fixtures for marks a recording cannot pose on demand.
fn staged(base: PathBuf, dir: &Path) -> Result<(), String> {
    for faction in [Faction::Union, Faction::Assembly] {
        let name = match faction {
            Faction::Union => "union",
            Faction::Assembly => "assembly",
            Faction::Compact => "compact",
        };
        let mut game = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-data")));
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        prepare(&mut game);
        let (own_line, enemy_line) = match faction {
            Faction::Union => (Kind::Bulwark, Kind::Loom),
            Faction::Assembly => (Kind::Loom, Kind::Bulwark),
            Faction::Compact => (Kind::Heliostat, Kind::Loom),
        };
        let (own_gun, enemy_gun) = match faction {
            Faction::Union => (Kind::Riveter, Kind::Reedguard),
            Faction::Assembly => (Kind::Reedguard, Kind::Riveter),
            Faction::Compact => (Kind::Brander, Kind::Reedguard),
        };
        // Packed, bracing, and deployed; ours above, theirs below; and a
        // melee in front.
        let mut place = |owner: u8, kind: Kind, cell: (i32, i32), state: u8| {
            let id = game
                .world
                .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
            if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) {
                e.facing = if owner == 0 { 2 } else { 6 };
                match state {
                    1 => {
                        e.deploy_target = true;
                        e.deploy_remaining = crate::presentation::DEPLOY_TICKS / 2;
                    }
                    2 => e.deployed = true,
                    _ => {}
                }
            }
        };
        for (i, state) in [0u8, 1, 2, 2].into_iter().enumerate() {
            place(0, own_line, (30 + i as i32 * 3, 58), state);
            place(1, enemy_line, (36 + i as i32 * 3, 64), state);
        }
        for i in 0..3 {
            place(0, own_gun, (33 + i * 2, 69), 0);
            place(1, enemy_gun, (34 + i * 2, 70), 0);
        }
        game.world.gate.owner = Some(0);
        for (zoom, label) in [(Zoom::Wide, "2x"), (Zoom::Overview, "1x")] {
            game.zoom = zoom;
            game.camera.center(Pos::cell(36, 63));
            game.selected.clear();
            game.message.clear();
            shot(&mut game, dir, &format!("staged-{name}-lines-{label}"))?;
        }
    }
    Ok(())
}
