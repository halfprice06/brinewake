//! Explicit offscreen art-state fixtures, separate from an interactive match.
//! Invoked only by --art-review. Does not save a game or alter simulation rules.
use crate::game::Game;
use bw_content::spec;
use bw_core::{FP, Faction, Kind, Pos};
use bw_sim::{Command, GATE_WARNING_TICKS, Order, Production, ResourceKind};
use std::path::Path;

pub fn export(game: &mut Game, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let appearances = [
        ("union_hq", Kind::Headquarters, Faction::Union),
        ("assembly_hq", Kind::Headquarters, Faction::Assembly),
        ("union_works", Kind::Works, Faction::Union),
        ("assembly_works", Kind::Works, Faction::Assembly),
        ("condenser", Kind::Condenser, Faction::Union),
        ("dropoff", Kind::Dropoff, Faction::Union),
        ("tower", Kind::Tower, Faction::Union),
    ];
    for (name, kind, faction) in appearances {
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        let mut subject = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .cloned()
            .ok_or("review requires starting headquarters")?;
        let mut worker = game
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .cloned()
            .ok_or("review requires starting worker")?;
        subject.kind = kind;
        subject.pos = Pos::cell(24, 60);
        subject.hp = spec(kind).health;
        subject.max_hp = subject.hp;
        subject.queue.clear();
        worker.pos = Pos::raw(subject.pos.x - FP * 2, subject.pos.y + FP * 3);
        game.camera.center(subject.pos);
        game.selected = vec![subject.id];
        game.message.clear();
        game.world.map.resources.clear();
        game.world.map.wells.clear();
        game.world.entities = vec![subject, worker];

        if spec(kind).build_ticks > 0 {
            for stage in 0..=3 {
                let total = spec(kind).build_ticks;
                game.world.entities[0].build_remaining = total - total * stage / 3;
                let done = total - game.world.entities[0].build_remaining;
                game.world.entities[0].hp = (spec(kind).health / 4
                    + spec(kind).health * done as i32 / total as i32)
                    .min(spec(kind).health);
                capture(
                    game,
                    &dir.join(format!("{name}-construction-{stage}.png")),
                    &format!("ART STATE STUDY: {name} / {stage}"),
                )?;
            }
        }
        if !matches!(kind, Kind::Dropoff | Kind::Tower) {
            game.world.entities[0].build_remaining = 0;
            game.world.entities[0].hp = spec(kind).health;
            if kind == Kind::Works {
                game.world.entities[0].queue = vec![Production {
                    kind: faction.army()[0],
                    remaining: 90,
                    started: true,
                    cost_salvage: 0,
                    cost_pressure: 0,
                }];
            }
            for phase in 0..4 {
                game.world.tick = phase * 6;
                capture(
                    game,
                    &dir.join(format!("{name}-activity-{phase}.png")),
                    &format!("ART STATE STUDY: {name} / ACTIVE"),
                )?;
            }
        }
    }
    export_v8_scenes(game, dir)?;
    Ok(())
}

fn export_v8_scenes(game: &mut Game, dir: &Path) -> Result<(), String> {
    export_gate_and_landmarks(game, dir)?;
    export_economy_states(game, dir)?;
    export_deployment_states(game, dir)?;
    export_combat_states(game, dir)?;
    Ok(())
}

fn reset_match(game: &mut Game, faction: Faction, center: Pos) {
    game.faction = faction;
    game.start();
    game.world.ai_enabled = false;
    game.camera.center(center);
    game.selected.clear();
    game.message.clear();
}

fn export_gate_and_landmarks(game: &mut Game, dir: &Path) -> Result<(), String> {
    for faction in [Faction::Union, Faction::Assembly] {
        reset_match(game, faction, game.world.map.gate_pos);
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm::from_north(true);
        capture(
            game,
            &dir.join(format!("v8-{faction:?}-gate-north-dry.png")),
            "V8 GATE / CURRENT N DRY S FLOODED",
        )?;
        game.world.gate.warning_until = Some(game.world.tick + u64::from(GATE_WARNING_TICKS));
        game.world.gate.switch_target = Some(bw_sim::Arm::SOUTH);
        capture(
            game,
            &dir.join(format!("v8-{faction:?}-gate-warning.png")),
            "V8 GATE / CURRENT TRUTH + TARGET POINTER",
        )?;
        game.world.gate.warning_until = None;
        game.world.gate.tide = bw_sim::Tide::Open;
        game.world.gate.dry_arm = bw_sim::Arm::from_north(false);
        game.gate_foam_start = Some(game.world.tick);
        for phase in 0u64..6 {
            game.world.tick = phase * 6;
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-gate-foam-{phase}.png")),
                "V8 GATE / ACTUAL SWITCH FOAM",
            )?;
        }
        for landmark in crate::presentation::LANDMARKS {
            let anchor = Pos::raw(landmark.pos.x + FP * 3, landmark.pos.y + FP * 2);
            if let Some(worker) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            {
                worker.pos = anchor;
            }
            game.camera.center(landmark.pos);
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-{}.png", landmark.key)),
                "V8 LANDMARK / COSMETIC DEPTH PROP",
            )?;
        }
    }
    Ok(())
}

fn export_economy_states(game: &mut Game, dir: &Path) -> Result<(), String> {
    for faction in [Faction::Union, Faction::Assembly] {
        reset_match(game, faction, Pos::cell(112, 57));
        let worker_id = game
            .world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .map(|entity| entity.id)
            .ok_or("v8 economy fixture requires a worker")?;
        game.selected = vec![worker_id];
        for (resource_index, resource_id) in [1u32, 2, 3].into_iter().enumerate() {
            let resource = game
                .world
                .map
                .resources
                .iter()
                .find(|resource| resource.id == resource_id)
                .cloned()
                .ok_or("v8 economy fixture requires three wrecks")?;
            game.camera.center(resource.pos);
            for (stage, remaining) in [(0u8, 2_400u32), (1, 1_200), (2, 600), (3, 0)] {
                game.world
                    .map
                    .resources
                    .iter_mut()
                    .find(|candidate| candidate.id == resource_id)
                    .expect("fixture resource")
                    .remaining = remaining;
                if let Some(worker) = game
                    .world
                    .entities
                    .iter_mut()
                    .find(|entity| entity.id == worker_id)
                {
                    worker.pos = Pos::raw(resource.pos.x + 2 * FP, resource.pos.y + 2 * FP);
                    worker.facing = 7;
                    worker.carried = if stage == 0 { 0 } else { 5 };
                    worker.carried_kind = (stage != 0).then_some(ResourceKind::Salvage);
                    worker.order = if stage == 0 {
                        Order::Gather {
                            resource: resource_id,
                        }
                    } else {
                        Order::Idle
                    };
                    worker.gather_ticks = if stage == 0 { 12 } else { 0 };
                }
                capture(
                    game,
                    &dir.join(format!(
                        "v8-{faction:?}-wreck-{resource_index}-stage-{stage}.png"
                    )),
                    &format!("V8 SALVAGE / WRECK {resource_index} / STAGE {stage}"),
                )?;
            }
        }
        // Loaded walking and the actual unload event receive dedicated state
        // samples so the two worker factions are reviewed at native scale.
        game.camera.center(Pos::cell(108, 57));
        if let Some(worker) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == worker_id)
        {
            worker.carried = 5;
            worker.carried_kind = Some(ResourceKind::Salvage);
            worker.pos = Pos::cell(108, 57);
            worker.order = Order::Move {
                target: Pos::cell(112, 57),
            };
        }
        game.motion
            .insert(worker_id, (Pos::cell(107, 57), 256, game.world.tick));
        capture(
            game,
            &dir.join(format!("v8-{faction:?}-worker-loaded-walk.png")),
            "V8 WORKER / LOADED WALK",
        )?;
        if let Some(worker) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == worker_id)
        {
            worker.carried = 0;
            worker.carried_kind = None;
            worker.order = Order::Idle;
        }
        game.unload_starts.insert(worker_id, game.world.tick);
        for phase in 0u64..3 {
            game.world.tick = phase * 3;
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-worker-unload-{phase}.png")),
                "V8 WORKER / UNLOAD POSE FIXTURE",
            )?;
        }
    }
    Ok(())
}

fn export_deployment_states(game: &mut Game, dir: &Path) -> Result<(), String> {
    for (faction, kind) in [
        (Faction::Union, Kind::Bulwark),
        (Faction::Assembly, Kind::Loom),
    ] {
        reset_match(game, faction, Pos::cell(24, 60));
        let id = game
            .world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind.is_worker())
            .map(|entity| entity.id)
            .ok_or("v8 deployment fixture requires a worker")?;
        if let Some(entity) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
        {
            entity.kind = kind;
            entity.hp = spec(kind).health;
            entity.max_hp = entity.hp;
            entity.pos = Pos::cell(24, 60);
        }
        game.selected = vec![id];
        for (stage, remaining, target, deployed) in [
            (0, 30, true, false),
            (1, 20, true, false),
            (2, 10, true, false),
            (3, 30, false, true),
            (4, 20, false, true),
            (5, 10, false, true),
            (6, 0, false, false),
            (7, 0, true, true),
        ] {
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == id)
            {
                entity.deploy_remaining = remaining;
                entity.deploy_target = target;
                entity.deployed = deployed;
                entity.order = Order::Deploy;
            }
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-{kind:?}-deploy-{stage}.png")),
                "V8 SPECIALIST / DEPLOYMENT STATE",
            )?;
        }
    }
    Ok(())
}

fn export_combat_states(game: &mut Game, dir: &Path) -> Result<(), String> {
    for (faction, roles) in [
        (Faction::Union, Faction::Union.army()),
        (Faction::Assembly, Faction::Assembly.army()),
    ] {
        for kind in roles {
            reset_match(game, faction, Pos::cell(48, 40));
            let attacker = game
                .world
                .entities
                .iter()
                .find(|entity| entity.owner == 0 && entity.kind.is_worker())
                .map(|entity| entity.id)
                .ok_or("v8 combat fixture requires an attacker")?;
            let target = game
                .world
                .entities
                .iter()
                .find(|entity| entity.owner == 1 && entity.kind.is_worker())
                .map(|entity| entity.id)
                .ok_or("v8 combat fixture requires a target")?;
            for entity in &mut game.world.entities {
                if entity.id == attacker {
                    entity.kind = kind;
                    entity.hp = spec(kind).health;
                    entity.max_hp = entity.hp;
                    entity.pos = Pos::cell(48, 40);
                    // Loom fire is a committed ground blast and requires the
                    // same completed deployment state as live play.  The
                    // older fixture expected an immediate Shot here, which
                    // left Assembly's Loom branch unable to produce art.
                    entity.deployed = kind == Kind::Loom;
                    entity.deploy_remaining = 0;
                    entity.deploy_target = entity.deployed;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.order = Order::Idle;
                } else if entity.id == target {
                    entity.pos = Pos::cell(51, 40);
                    entity.kind = if faction == Faction::Union {
                        Kind::Reedguard
                    } else {
                        Kind::Bulwark
                    };
                    entity.hp = spec(entity.kind).health;
                    entity.max_hp = entity.hp;
                    entity.deployed = entity.kind == Kind::Bulwark;
                    entity.facing = 6;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.order = Order::Hold;
                }
            }
            game.selected = vec![attacker];
            game.world
                .issue(
                    0,
                    Command::Attack {
                        units: vec![attacker],
                        target,
                    },
                )
                .map_err(|e| format!("v8 combat fixture attack: {e}"))?;
            game.tick();
            let expected_fire = if kind == Kind::Loom {
                bw_sim::EventKind::ArtilleryWarning
            } else {
                bw_sim::EventKind::Shot
            };
            if !game
                .world
                .events
                .iter()
                .any(|event| event.kind == expected_fire && event.entity == Some(attacker))
            {
                return Err(format!(
                    "{faction:?} {kind:?} fixture did not actually fire"
                ));
            }
            if kind == Kind::Loom {
                capture(
                    game,
                    &dir.join(format!("v8-{faction:?}-{kind:?}-fire-warning.png")),
                    "V8 COMBAT / LOOM COMMITTED WARNING",
                )?;
                while !game.world.artillery.is_empty() {
                    game.tick();
                }
                capture(
                    game,
                    &dir.join(format!("v8-{faction:?}-{kind:?}-fire-impact.png")),
                    "V8 COMBAT / LOOM SHOT IMPACT + TARGET MATERIAL",
                )?;
            } else {
                capture(
                    game,
                    &dir.join(format!("v8-{faction:?}-{kind:?}-fire-impact.png")),
                    "V8 COMBAT / SHOT RECOIL + TARGET MATERIAL",
                )?;
            }
            // The target is deliberately one hit from death, so this second
            // authoritative event creates an observed faction residue.
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == attacker)
            {
                entity.attack_cooldown = 0;
            }
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == target)
            {
                entity.hp = 1;
            }
            game.world.tick += 31;
            game.world
                .issue(
                    0,
                    Command::Attack {
                        units: vec![attacker],
                        target,
                    },
                )
                .map_err(|e| format!("v8 combat fixture death: {e}"))?;
            game.tick();
            if kind == Kind::Loom {
                if !game.world.events.iter().any(|event| {
                    event.kind == bw_sim::EventKind::ArtilleryWarning
                        && event.entity == Some(attacker)
                }) {
                    return Err(format!(
                        "{faction:?} {kind:?} fixture did not commit its second shot"
                    ));
                }
                while !game.world.artillery.is_empty() {
                    game.tick();
                }
            }
            if !game
                .world
                .events
                .iter()
                .any(|event| event.kind == bw_sim::EventKind::Death && event.entity == Some(target))
            {
                return Err(format!(
                    "{faction:?} {kind:?} fixture did not destroy its target"
                ));
            }
            // Separate the living attacker from the observed residue in the
            // review frame; this is a cosmetic fixture adjustment after the
            // authoritative death event, not a gameplay move.
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == attacker)
            {
                entity.pos = Pos::cell(45, 40);
            }
            game.selected.clear();
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-{kind:?}-wreck.png")),
                "V8 COMBAT / OBSERVED BREAKUP",
            )?;
            game.world.tick += crate::presentation::WRECK_TICKS;
            capture(
                game,
                &dir.join(format!("v8-{faction:?}-{kind:?}-residue.png")),
                "V8 COMBAT / OBSERVED GROUND DEBRIS",
            )?;
        }
    }
    Ok(())
}

fn capture(game: &mut Game, path: &Path, label: &str) -> Result<(), String> {
    game.render();
    game.canvas.rect(6, 273, 628, 13, [20, 33, 42, 255]);
    game.canvas.text(label, 11, 276, [222, 197, 150, 255]);
    game.canvas.save(path)
}
