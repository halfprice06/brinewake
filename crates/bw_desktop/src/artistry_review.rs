//! Controlled artistry fixtures for the desktop renderer.
//!
//! These scenarios use a fresh `Game` and explicit authoritative fields so a
//! reviewer can inspect the new art at native resolution and at actual tick
//! boundaries.  They are fixture evidence, not human playtest, balance, or
//! final-appeal evidence.  Root can invoke [`practice`] for a native field or
//! [`export`] for a labelled contact sheet of the same scenarios.

use crate::game::Game;
use bw_content::{Doctrine, SURGE_COOLDOWN_TICKS, SURGE_TICKS, spec};
use bw_core::{Faction, Kind, Pos};
use bw_sim::{Command, Entity, Order};
use std::path::Path;

const SURGE_REMAINING_BY_PHASE: [u32; 4] = [SURGE_TICKS, 87, 84, 81];
const COOL_REMAINING_BY_PHASE: [u32; 3] = [SURGE_COOLDOWN_TICKS, 240, 120];
const LOOM_FRAME_TICKS: [u64; 6] = [1, 5, 9, 13, 17, 21];

/// Prepare a fresh, labelled practice field with all six combat roles, one
/// local worker, a completed local doctrine, and quiet public shore sites.
///
/// The field is deterministic and has no AI.  It is deliberately assembled
/// from the normal `World::new` state and then rebased with
/// `reset_fixture_origin`, so a reviewer can still inspect save/replay state
/// without the art fixture changing simulation rules.
pub fn practice(game: &mut Game) -> Result<(), String> {
    game.start();
    game.world.ai_enabled = false;
    game.ux.practice = true;
    game.ux.tactics_fixture = true;
    game.ux.practice_review = false;
    game.ux.practice_complete = false;
    game.ux.tutorial = None;
    game.ux.guidance_visible = false;
    game.message.clear();
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
    game.world.gate.warning_until = None;
    game.world.gate.switch_target = None;
    game.world.gate.lane_revision = 0;
    game.world.lane_revision = 0;
    game.world.players[0].doctrine = Some(Doctrine::Hauling);
    game.world.players[0].research = None;
    game.world.players[0].salvage = 1_500;
    game.world.players[0].pressure = 300;

    let worker_ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.owner == 0 && entity.kind.is_worker())
        .map(|entity| entity.id)
        .collect();
    let template = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .cloned()
        .ok_or("artistry fixture requires a local worker")?;
    let worker_kind = game.world.players[0].faction.worker();
    let placements = [
        (worker_kind, (40, 44)),
        (Kind::Riveter, (56, 49)),
        (Kind::Bulwark, (60, 49)),
        (Kind::Sounder, (69, 49)),
        (Kind::Skipper, (56, 79)),
        (Kind::Reedguard, (60, 79)),
        (Kind::Loom, (69, 79)),
    ];
    let mut ids = worker_ids;
    let mut next_id = game
        .world
        .entities
        .iter()
        .map(|entity| entity.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    while ids.len() < placements.len() {
        let mut clone = template.clone();
        clone.id = next_id;
        next_id = next_id.saturating_add(1);
        game.world.entities.push(clone);
        ids.push(next_id.saturating_sub(1));
    }
    for (index, (id, (kind, (x, y)))) in ids.iter().copied().zip(placements).enumerate() {
        let entity = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .ok_or("artistry fixture entity disappeared")?;
        reset_entity(entity, kind, Pos::cell(x, y), index as u8);
    }
    game.world.players[0].crew = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.owner == 0)
        .map(|entity| spec(entity.kind).crew)
        .sum();
    game.selected = ids
        .iter()
        .copied()
        .filter(|id| {
            game.world
                .entities
                .iter()
                .any(|entity| entity.id == *id && !entity.kind.is_worker())
        })
        .collect();
    game.groups[1] = game.selected.clone();
    game.camera.center(Pos::cell(62, 64));
    game.world.reset_fixture_origin()?;
    game.notify("CONTROLLED ARTISTRY FIELD / TIDE, PRESSURE, DOCTRINE, COAST");
    Ok(())
}

/// Export labelled native-resolution fixtures from one caller-owned `Game`.
/// Each capture is made after a normal render; changing a fixture clock is
/// explicit and bounded so the output remains reproducible and easy to audit.
pub fn export(game: &mut Game, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    practice(game)?;

    game.camera.center(Pos::cell(62, 64));
    capture(game, dir, "01-field", "ARTISTRY / FRESH CONTROLLED FIELD")?;

    // Both lane states are sampled at the same authoritative tick.  The
    // public gate field, not a presentation timer, chooses dry versus wet.
    game.camera.center(Pos::cell(58, 49));
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
    capture(
        game,
        dir,
        "02-north-dry",
        "ARCHAEOLOGY / NORTH FERRY COURT / DRY",
    )?;
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(false);
    capture(
        game,
        dir,
        "03-north-wet-same-tick",
        "ARCHAEOLOGY / NORTH FERRY COURT / WET SAME TICK",
    )?;
    game.world.gate.tide = bw_sim::Tide::Open;
    game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
    game.camera.center(Pos::cell(58, 79));
    capture(
        game,
        dir,
        "04-south-wet",
        "ARCHAEOLOGY / SOUTH ROPEWALK / WET",
    )?;

    // Center each shore vignette separately.  The map cannot fit all three
    // edge sites at once in the 640x360 viewport, so this keeps every authored
    // ecology site directly reachable in the review output.
    let coast_observer = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 0 && entity.kind.is_worker())
        .map(|entity| entity.id)
        .ok_or("coast fixture requires a local observer")?;
    game.selected.clear();
    for (index, site) in crate::artistry::COAST_SITES.iter().enumerate() {
        // Keep the observer four cells away: close enough for a bright,
        // visible vignette, outside the site's two-cell occlusion guard.
        if let Some(observer) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == coast_observer)
        {
            observer.pos = Pos::cell(site.pos.0, site.pos.1 + 4);
        }
        game.camera.center(Pos::cell(site.pos.0, site.pos.1));
        capture(
            game,
            dir,
            &format!("05-coast-{index}"),
            &format!("COAST / {} / QUIET HOLD", site.name.to_ascii_uppercase()),
        )?;
    }

    let combat_ids = combat_ids(game);
    // Frame each role at its own ground position.  A gate-centred overview
    // leaves most selected units outside the viewport and cannot establish
    // that every physical pressure overlay is attached to its machine.
    for id in &combat_ids {
        let (role, pos) = game
            .world
            .entities
            .iter()
            .find(|entity| entity.id == *id)
            .map(|entity| (entity.kind.asset(game.world.players[0].faction), entity.pos))
            .ok_or("pressure fixture entity disappeared")?;
        game.selected = vec![*id];
        game.camera.center(pos);
        for (phase, remaining) in SURGE_REMAINING_BY_PHASE.into_iter().enumerate() {
            set_pressure(game, &combat_ids, remaining, 0);
            capture(
                game,
                dir,
                &format!("06-pressure-{role}-surge-{phase}"),
                &format!("PRESSURE / {role} / SURGE PHASE {phase}"),
            )?;
        }
        for (phase, remaining) in COOL_REMAINING_BY_PHASE.into_iter().enumerate() {
            set_pressure(game, &combat_ids, 0, remaining);
            capture(
                game,
                dir,
                &format!("06-pressure-{role}-cool-{phase}"),
                &format!("PRESSURE / {role} / COOL PHASE {phase}"),
            )?;
        }
    }

    // Every authored role/facing receives one active and one cooling sample.
    // This catches atlas orientation or anchor mistakes that a six-unit
    // overview can hide while keeping the fixture entirely presentation-only.
    for id in &combat_ids {
        let (role, pos) = game
            .world
            .entities
            .iter()
            .find(|entity| entity.id == *id)
            .map(|entity| (entity.kind.asset(game.world.players[0].faction), entity.pos))
            .ok_or("pressure fixture entity disappeared")?;
        game.camera.center(pos);
        game.selected = vec![*id];
        for face in 0..8u8 {
            set_pressure(game, &combat_ids, 0, 0);
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == *id)
            {
                entity.facing = face;
                entity.surge_remaining = SURGE_TICKS;
                entity.surge_cooldown = SURGE_COOLDOWN_TICKS;
            }
            capture(
                game,
                dir,
                &format!("07b-pressure-{role}-face-{face}-surge"),
                &format!("PRESSURE / {role} / FACE {face} / SURGE"),
            )?;
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == *id)
            {
                entity.surge_remaining = 0;
                entity.surge_cooldown = SURGE_COOLDOWN_TICKS;
            }
            capture(
                game,
                dir,
                &format!("07b-pressure-{role}-face-{face}-cool"),
                &format!("PRESSURE / {role} / FACE {face} / COOL"),
            )?;
        }
    }
    write_art_key_audit(game, dir)?;

    // Restore a clean field before using an actual committed Loom shot.  The
    // warning is produced by ordinary command validation and world stepping;
    // the frame strip samples the existing 24-tick windup without inventing a
    // hidden source or an extra impact delay.
    practice(game)?;
    let loom = prepare_loom_shot(game)?;
    let loom_pos = game
        .world
        .entities
        .iter()
        .find(|entity| entity.id == loom)
        .map(|entity| entity.pos)
        .ok_or("Loom disappeared from artistry fixture")?;
    game.camera.center(loom_pos);
    for (phase, sample_tick) in LOOM_FRAME_TICKS.into_iter().enumerate() {
        while game.world.tick < sample_tick {
            game.tick();
        }
        capture(
            game,
            dir,
            &format!("08-loom-pressure-{phase}"),
            &format!("LOOM PRESSURE / COMMITTED WINDUP / FRAME {phase}"),
        )?;
    }
    while !game.world.artillery.is_empty() {
        game.tick();
    }
    capture(
        game,
        dir,
        "09-loom-impact",
        "LOOM PRESSURE / AUTHORITATIVE IMPACT",
    )?;

    // Doctrine plates are easiest to judge at their authored HQ and worker
    // scales.  Exercise both factions and both completed HQ choices; worker
    // cargo attachments exist for the authored Hauling kit only.
    for faction in [Faction::Union, Faction::Assembly, Faction::Compact] {
        game.faction = faction;
        practice(game)?;
        let faction_key = match faction {
            Faction::Union => "union",
            Faction::Assembly => "assembly",
            Faction::Compact => "compact",
        };
        let hq_id = game
            .world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
            .map(|entity| entity.id)
            .ok_or("HQ disappeared from artistry fixture")?;
        let hq_pos = game
            .world
            .entities
            .iter()
            .find(|entity| entity.id == hq_id)
            .map(|entity| entity.pos)
            .ok_or("HQ position disappeared from artistry fixture")?;
        for doctrine in [Doctrine::Hauling, Doctrine::FireControl] {
            game.world.players[0].doctrine = Some(doctrine);
            game.selected = vec![hq_id];
            game.camera.center(hq_pos);
            let doctrine_key = match doctrine {
                Doctrine::Hauling => "hauling",
                Doctrine::FireControl => "fire-control",
            };
            capture(
                game,
                dir,
                &format!("10-doctrine-{faction_key}-hq-{doctrine_key}"),
                &format!(
                    "DOCTRINE / {} HQ / {} COMPLETE",
                    faction.name(),
                    doctrine_key.to_ascii_uppercase()
                ),
            )?;
            if doctrine == Doctrine::Hauling {
                let worker_id = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| entity.owner == 0 && entity.kind == faction.worker())
                    .map(|entity| entity.id)
                    .ok_or("worker disappeared from artistry fixture")?;
                let worker_pos = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| entity.id == worker_id)
                    .map(|entity| entity.pos)
                    .ok_or("worker position disappeared from artistry fixture")?;
                game.selected = vec![worker_id];
                game.camera.center(worker_pos);
                capture(
                    game,
                    dir,
                    &format!("11-doctrine-{faction_key}-worker-hauling"),
                    &format!("DOCTRINE / {} / HAULING COMPLETE", faction.worker().name()),
                )?;
            }
        }
    }

    std::fs::write(
        dir.join("evidence.txt"),
        "Controlled artistry fixtures: four tide archaeology patches in dry/wet states, all three coast sites centered for direct inspection, per-role Surge/cooling phase strips, every combat-role/facing active and cooling pair, a committed Loom 24-tick warning strip and impact, plus completed owner-only doctrine overlays for both factions and HQ choices. Captures are native 640x360 renderer fixtures, not human playtest or appeal evidence.\n",
    )
    .map_err(|error| error.to_string())
}

fn reset_entity(entity: &mut Entity, kind: Kind, pos: Pos, facing: u8) {
    entity.kind = kind;
    entity.pos = pos;
    entity.hp = spec(kind).health;
    entity.max_hp = entity.hp;
    entity.facing = facing % 8;
    entity.build_remaining = 0;
    entity.queue.clear();
    entity.deployed = false;
    entity.deploy_remaining = 0;
    entity.deploy_target = false;
    entity.surge_remaining = 0;
    entity.surge_cooldown = 0;
    entity.order = Order::Idle;
    entity.carried = 0;
    entity.carried_kind = None;
    entity.gather_ticks = 0;
    entity.attack_cooldown = 0;
    entity.path.clear();
    entity.path_index = 0;
    entity.path_target = None;
    entity.path_lane_revision = 0;
    entity.blocked_ticks = 0;
    entity.waypoints.clear();
    entity.rally = None;
    entity.builder = None;
    entity.last_seen = None;
}

fn combat_ids(game: &Game) -> Vec<u32> {
    game.world
        .entities
        .iter()
        .filter(|entity| {
            entity.owner == 0 && !entity.kind.is_worker() && !entity.kind.is_building()
        })
        .map(|entity| entity.id)
        .collect()
}

fn set_pressure(game: &mut Game, ids: &[u32], surge_remaining: u32, cooldown: u32) {
    let surge_elapsed = SURGE_TICKS.saturating_sub(surge_remaining);
    let active_cooldown = SURGE_COOLDOWN_TICKS.saturating_sub(surge_elapsed);
    let cooldown = if surge_remaining > 0 {
        active_cooldown
    } else {
        cooldown
    };
    for id in ids {
        if let Some(entity) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == *id)
        {
            entity.surge_remaining = surge_remaining;
            entity.surge_cooldown = cooldown;
        }
    }
}

fn prepare_loom_shot(game: &mut Game) -> Result<u32, String> {
    let loom = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 0 && entity.kind == Kind::Loom)
        .map(|entity| entity.id)
        .ok_or("artillery fixture requires a local Loom")?;
    let target = game
        .world
        .entities
        .iter()
        .find(|entity| entity.owner == 1 && entity.kind.is_worker())
        .map(|entity| entity.id)
        .ok_or("artillery fixture requires an opponent target")?;
    if let Some(entity) = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.id == loom)
    {
        entity.pos = Pos::cell(40, 64);
        entity.deployed = true;
        entity.deploy_remaining = 0;
        entity.deploy_target = true;
        entity.surge_remaining = 0;
        entity.surge_cooldown = 0;
        entity.attack_cooldown = 0;
        entity.order = Order::Idle;
    }
    if let Some(entity) = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.id == target)
    {
        entity.pos = Pos::cell(45, 64);
        entity.hp = spec(entity.kind).health;
        entity.max_hp = entity.hp;
        entity.order = Order::Idle;
        entity.path.clear();
        entity.path_index = 0;
        entity.path_target = None;
    }
    game.selected = vec![loom];
    game.camera.center(Pos::cell(40, 64));
    game.world
        .issue(
            0,
            Command::Attack {
                units: vec![loom],
                target,
            },
        )
        .map_err(|error| format!("artillery fixture attack: {error}"))?;
    game.tick();
    if !game.world.events.iter().any(|event| {
        event.kind == bw_sim::EventKind::ArtilleryWarning && event.entity == Some(loom)
    }) {
        return Err("deployed Loom did not commit an artillery warning".into());
    }
    if game.world.artillery.is_empty() {
        return Err("deployed Loom warning has no pending shot".into());
    }
    Ok(loom)
}

fn capture(game: &mut Game, dir: &Path, name: &str, label: &str) -> Result<(), String> {
    game.render();
    game.canvas.rect(6, 273, 628, 13, [20, 33, 42, 255]);
    game.canvas.text(label, 11, 276, [222, 197, 150, 255]);
    game.canvas.save(&dir.join(format!("{name}.png")))
}

fn write_art_key_audit(game: &Game, dir: &Path) -> Result<(), String> {
    let mut lines = Vec::new();
    let Some(atlas) = game.atlas.as_ref() else {
        lines.push("atlas unavailable; key audit deferred until asset load is repaired".into());
        return std::fs::write(dir.join("key-audit.txt"), lines.join("\n"))
            .map_err(|error| error.to_string());
    };
    for lane in ["north", "south"] {
        for state in ["dry", "wet"] {
            for phase in 0..4 {
                let key = format!("archaeology_{lane}_{state}_{phase}");
                lines.push(format!(
                    "{key}: {}",
                    sprite_status(atlas, &key, (160, 80), (80, 40))
                ));
            }
        }
    }
    for role in ["birds", "crab", "reeds"] {
        for phase in 0..8 {
            let key = format!("coast_{role}_{phase}");
            lines.push(format!(
                "{key}: {}",
                sprite_status(atlas, &key, (96, 64), (48, 52))
            ));
        }
    }
    for role in [
        "riveter",
        "bulwark",
        "sounder",
        "skipper",
        "reedguard",
        "loom",
    ] {
        for face in 0..8 {
            for (mode, count) in [("surge", 4), ("cool", 3)] {
                for phase in 0..count {
                    let key = format!("pressure_{role}_{face}_{mode}_{phase}");
                    lines.push(format!(
                        "{key}: {}",
                        sprite_status(atlas, &key, (64, 64), (32, 50))
                    ));
                }
            }
        }
    }
    for face in 0..8 {
        for phase in 0..6 {
            let key = format!("loom_{face}_pressure_{phase}");
            lines.push(format!(
                "{key}: {}",
                sprite_status(atlas, &key, (64, 64), (32, 50))
            ));
        }
    }
    for faction in ["union", "assembly", "compact"] {
        for doctrine in ["hauling", "fire_control"] {
            for phase in 0..4 {
                let key = format!("doctrine_{faction}_{doctrine}_hq_{phase}");
                lines.push(format!(
                    "{key}: {}",
                    sprite_status(atlas, &key, (160, 144), (80, 128))
                ));
            }
        }
    }
    for role in ["hook", "wick", "raker"] {
        for face in 0..8 {
            for phase in 0..4 {
                let key = format!("doctrine_{role}_{face}_hauling_{phase}");
                lines.push(format!(
                    "{key}: {}",
                    sprite_status(atlas, &key, (64, 64), (32, 50))
                ));
            }
        }
    }
    lines.push(String::new());
    lines.push("The fixture frames are native 640x360 captures; this audit does not claim human appeal or final art approval.".into());
    std::fs::write(dir.join("key-audit.txt"), lines.join("\n")).map_err(|error| error.to_string())
}

fn sprite_status(
    atlas: &crate::canvas::Atlas,
    key: &str,
    size: (u32, u32),
    anchor: (i32, i32),
) -> String {
    atlas.sprites.get(key).map_or_else(
        || "MISSING".to_owned(),
        |sprite| {
            if (sprite.w, sprite.h) == size && (sprite.anchor_x, sprite.anchor_y) == anchor {
                "OK".to_owned()
            } else {
                format!(
                    "BAD_DIM {}x{}@{},{}",
                    sprite.w, sprite.h, sprite.anchor_x, sprite.anchor_y
                )
            }
        },
    )
}
