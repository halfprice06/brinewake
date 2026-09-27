use bw_core::{Faction, Kind, Pos, Terrain};
use bw_sim::{Command, EventKind, Outcome, World};
use serde_json::{Value, json};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn ids_for(world: &World, owner: u8, kind: Kind) -> Vec<u32> {
    world
        .entities
        .iter()
        .filter(|entity| entity.owner == owner && entity.kind == kind && entity.hp > 0)
        .map(|entity| entity.id)
        .collect()
}

fn hq(world: &World, owner: u8) -> (u32, Pos) {
    world
        .entities
        .iter()
        .find(|entity| entity.owner == owner && entity.kind == Kind::Headquarters)
        .map(|entity| (entity.id, entity.pos))
        .expect("each player starts with a headquarters")
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "brinewake-system-{}-{label}-{nanos}.json",
        std::process::id()
    ))
}

fn entity_state(world: &World, id: u32) -> String {
    world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| {
            format!(
                "id={id} kind={:?} cell={:?} hp={} order={:?} path={}/{} blocked={}",
                entity.kind,
                entity.pos.cell_xy(),
                entity.hp,
                entity.order,
                entity.path_index,
                entity.path.len(),
                entity.blocked_ticks
            )
        })
        .unwrap_or_else(|| format!("id={id} gone"))
}

fn build_works(world: &mut World, owner: u8, worker: u32, pos: Pos) -> u32 {
    world
        .issue(
            owner,
            Command::Build {
                worker,
                kind: Kind::Works,
                pos,
                queued: false,
            },
        )
        .expect("Works build command should be accepted");

    let works =
        loop {
            if let Some(entity) = world.entities.iter().find(|entity| {
                entity.owner == owner && entity.kind == Kind::Works && entity.pos == pos
            }) {
                break entity.id;
            }
            world.step();
            assert!(world.tick < 4_000, "Works was never spawned");
        };
    for _ in 0..4_000 {
        if world
            .entities
            .iter()
            .any(|entity| entity.id == works && entity.build_remaining == 0)
        {
            return works;
        }
        world.step();
    }
    panic!(
        "Works {works} did not finish construction by tick {}",
        world.tick
    );
}

fn worker_resource(world: &World, owner: u8) -> u32 {
    let x_limit = if owner == 0 { 40 } else { 88 };
    world
        .map
        .resources
        .iter()
        .find(|resource| {
            resource.remaining > 0
                && if owner == 0 {
                    resource.pos.cell_xy().0 < x_limit
                } else {
                    resource.pos.cell_xy().0 > x_limit
                }
        })
        .map(|resource| resource.id)
        .expect("each side has a starting salvage resource")
}

#[test]
fn six_workers_collect_and_deposit_repeated_loads() {
    let mut world = World::new(101, Faction::Union);
    world.ai_enabled = false;
    assert_eq!(ids_for(&world, 0, Kind::Hook).len(), 6);
    assert_eq!(ids_for(&world, 0, Kind::Headquarters).len(), 1);
    let workers = ids_for(&world, 0, Kind::Hook);
    let resource = worker_resource(&world, 0);
    let starting_salvage = world.players[0].salvage;

    world
        .issue(
            0,
            Command::Gather {
                units: workers,
                resource,
            },
        )
        .expect("six-worker gather command should be accepted");

    let mut deposits = 0usize;
    let mut gathers = 0usize;
    for _ in 0..2_400 {
        world.step();
        deposits += world
            .events
            .iter()
            .filter(|event| event.player == Some(0) && event.kind == EventKind::Deposit)
            .count();
        gathers += world
            .events
            .iter()
            .filter(|event| event.player == Some(0) && event.kind == EventKind::Gather)
            .count();
        if deposits >= 12 {
            break;
        }
    }

    assert!(
        gathers >= 12,
        "workers did not complete repeated loads: gathers={gathers}, deposits={deposits}, tick={}, resource_remaining={}, states=[{}]",
        world.tick,
        world
            .map
            .resources
            .iter()
            .find(|item| item.id == resource)
            .map(|item| item.remaining)
            .unwrap_or_default(),
        ids_for(&world, 0, Kind::Hook)
            .iter()
            .map(|id| entity_state(&world, *id))
            .collect::<Vec<_>>()
            .join("; ")
    );
    assert!(
        deposits >= 12,
        "six workers did not deposit repeated loads within 80 seconds: deposits={deposits}, tick={}",
        world.tick
    );
    assert!(world.players[0].salvage > starting_salvage);
}

#[test]
fn build_train_and_real_damage_can_destroy_enemy_headquarters() {
    let mut world = World::new(102, Faction::Union);
    world.ai_enabled = false;
    let worker = ids_for(&world, 0, Kind::Hook)[0];
    let works = build_works(&mut world, 0, worker, Pos::cell(25, 60));
    let (enemy_hq_id, enemy_hq_pos) = hq(&world, 1);

    let mut sent = std::collections::BTreeSet::new();
    let mut damage_events = 0usize;
    let mut accepted_train_attempt = false;
    for _ in 0..18_000 {
        let queue_count = world
            .entities
            .iter()
            .find(|entity| entity.id == works)
            .map(|entity| {
                entity
                    .queue
                    .iter()
                    .filter(|item| item.kind == Kind::Riveter)
                    .count()
            })
            .unwrap_or(usize::MAX);
        let riveters = ids_for(&world, 0, Kind::Riveter);
        if riveters.len() + queue_count < 12
            && queue_count < 12
            && world
                .issue(
                    0,
                    Command::Train {
                        building: works,
                        kind: Kind::Riveter,
                    },
                )
                .is_ok()
        {
            accepted_train_attempt = true;
        }

        world.step();
        let current_riveters = ids_for(&world, 0, Kind::Riveter);
        for id in current_riveters {
            if sent.insert(id) {
                world
                    .issue(
                        0,
                        Command::AttackMove {
                            units: vec![id],
                            target: enemy_hq_pos,
                            queued: false,
                        },
                    )
                    .expect("new Riveter attack-move should be accepted");
            }
        }
        damage_events += world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Damage && event.entity == Some(enemy_hq_id))
            .count();
        if world.outcome == Some(Outcome::Victory(0)) {
            break;
        }
    }

    assert!(accepted_train_attempt, "no Riveter was ever queued");
    assert!(damage_events > 0, "enemy headquarters took no real damage");
    assert_eq!(world.outcome, Some(Outcome::Victory(0)));
}

fn route_probe(world: &mut World, id: u32, target: Pos) -> (Vec<Terrain>, u64) {
    let mut route = Vec::new();
    let start_tick = world.tick;
    for _ in 0..12_000 {
        world.step();
        let Some(entity) = world.entities.iter().find(|entity| entity.id == id) else {
            panic!("route probe worker disappeared");
        };
        if route.is_empty() {
            route = entity
                .path
                .iter()
                .map(|pos| world.map.terrain(pos.cell_xy().0, pos.cell_xy().1))
                .collect();
        }
        if entity.pos == target {
            return (route, world.tick - start_tick);
        }
    }
    panic!("route probe did not arrive by tick {}", world.tick);
}

fn route_lanes(route: &[Terrain]) -> (usize, usize) {
    let mut north = 0usize;
    let mut south = 0usize;
    for terrain in route {
        match terrain {
            Terrain::Lane0 => north += 1,
            Terrain::Lane1 => south += 1,
            _ => {}
        }
    }
    (north, south)
}

#[test]
fn gate_capture_emits_once_and_both_lanes_have_reachable_distinct_costs() {
    let mut world = World::new(103, Faction::Union);
    world.ai_enabled = false;
    let worker = ids_for(&world, 0, Kind::Hook)[0];
    let works = build_works(&mut world, 0, worker, Pos::cell(25, 60));
    world
        .issue(
            0,
            Command::Train {
                building: works,
                kind: Kind::Sounder,
            },
        )
        .expect("Sounder train command should be accepted");
    let sounder = loop {
        let ids = ids_for(&world, 0, Kind::Sounder);
        if let Some(id) = ids.first() {
            break *id;
        }
        world.step();
        assert!(world.tick < 4_000, "Sounder was never produced");
    };
    let gate = world.map.gate_pos;
    world
        .issue(
            0,
            Command::Move {
                units: vec![sounder],
                target: gate,
                queued: false,
            },
        )
        .expect("Sounder move to gate should be accepted");
    for _ in 0..4_000 {
        world.step();
        if world
            .entities
            .iter()
            .find(|entity| entity.id == sounder)
            .is_some_and(|entity| entity.pos.distance_sq(gate) <= i64::from(bw_core::FP * 2).pow(2))
        {
            break;
        }
    }
    assert!(world
        .entities
        .iter()
        .find(|entity| entity.id == sounder)
        .is_some_and(|entity| entity.pos.distance_sq(gate) <= i64::from(bw_core::FP * 2).pow(2)));

    world
        .issue(
            0,
            Command::Capture {
                units: vec![sounder],
            },
        )
        .expect("Sounder capture command should be accepted");
    let mut captured_events = 0usize;
    for _ in 0..(world.capture_work() + 60) {
        world.step();
        captured_events += world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::GateCaptured && event.player == Some(0))
            .count();
        if world.gate.owner == Some(0) && captured_events == 1 {
            break;
        }
    }
    assert_eq!(world.gate.owner, Some(0));
    assert_eq!(captured_events, 1, "capture should emit exactly once");

    world
        .issue(
            0,
            Command::Capture {
                units: vec![sounder],
            },
        )
        .expect("re-capture command should be accepted");
    for _ in 0..60 {
        world.step();
        captured_events += world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::GateCaptured && event.player == Some(0))
            .count();
    }
    assert_eq!(
        captured_events, 1,
        "holding an owned gate must not recapture it"
    );

    let mut dry = World::new(104, Faction::Union);
    let mut flooded = dry.clone();
    dry.ai_enabled = false;
    flooded.ai_enabled = false;
    // The tide is neutral at the start; open it one way and the other.
    dry.gate.tide = bw_sim::Tide::Open;
    dry.gate.dry_arm = bw_sim::Arm::from_north(true);
    flooded.gate.tide = bw_sim::Tide::Open;
    flooded.gate.dry_arm = bw_sim::Arm::from_north(false);
    let dry_worker = ids_for(&dry, 0, Kind::Hook)[0];
    let flooded_worker = ids_for(&flooded, 0, Kind::Hook)[0];
    let destination = Pos::cell(100, 64);
    dry.issue(
        0,
        Command::Move {
            units: vec![dry_worker],
            target: destination,
            queued: false,
        },
    )
    .expect("dry route probe should be accepted");
    flooded
        .issue(
            0,
            Command::Move {
                units: vec![flooded_worker],
                target: destination,
                queued: false,
            },
        )
        .expect("flooded route probe should be accepted");
    let (dry_path, dry_ticks) = route_probe(&mut dry, dry_worker, destination);
    let (flooded_path, flooded_ticks) = route_probe(&mut flooded, flooded_worker, destination);
    let dry_route = route_lanes(&dry_path);
    let flooded_route = route_lanes(&flooded_path);
    assert!(
        dry_route.0 > 0 || dry_route.1 > 0,
        "dry state needs a shallow route"
    );
    assert!(
        flooded_route.0 > 0 || flooded_route.1 > 0,
        "flooded state needs a shallow route"
    );
    assert_ne!(
        dry_ticks, flooded_ticks,
        "lane state must change actual travel time"
    );
    assert!(
        (dry_route.0 > 0 && flooded_route.1 > 0) || (dry_route.1 > 0 && flooded_route.0 > 0),
        "the two state probes should use opposite shallow lanes: dry={dry_route:?}, flooded={flooded_route:?}"
    );
}

#[test]
fn save_and_resume_preserve_deterministic_future_state() {
    let mut world = World::new(105, Faction::Assembly);
    world.ai_enabled = false;
    let workers = ids_for(&world, 0, Kind::Wick);
    let resource = worker_resource(&world, 0);
    world
        .issue(
            0,
            Command::Gather {
                units: vec![workers[0]],
                resource,
            },
        )
        .expect("gather should be accepted before save");
    world
        .issue(
            0,
            Command::Move {
                units: vec![workers[1]],
                target: Pos::cell(30, 66),
                queued: false,
            },
        )
        .expect("move should be accepted before save");

    let path = temp_path("resume");
    world
        .save(&path)
        .expect("save should succeed with pending commands");
    let mut loaded = World::load(&path).expect("saved world should load");
    assert_eq!(world.state_hash(), loaded.state_hash());
    for _ in 0..600 {
        world.step();
        loaded.step();
        assert_eq!(
            world.state_hash(),
            loaded.state_hash(),
            "diverged at tick {}",
            world.tick
        );
    }
    let _ = fs::remove_file(path);
}

#[test]
fn entity_ids_never_reuse_after_real_death_and_production() {
    let mut world = World::new(106, Faction::Union);
    world.ai_enabled = false;
    let worker0 = ids_for(&world, 0, Kind::Hook)[0];
    let worker1 = ids_for(&world, 1, Kind::Wick)[0];
    let works0 = build_works(&mut world, 0, worker0, Pos::cell(25, 60));
    let works1 = build_works(&mut world, 1, worker1, Pos::cell(102, 90));
    world
        .issue(
            0,
            Command::Train {
                building: works0,
                kind: Kind::Riveter,
            },
        )
        .expect("Union unit train command should be accepted");
    world
        .issue(
            1,
            Command::Train {
                building: works1,
                kind: Kind::Reedguard,
            },
        )
        .expect("Assembly unit train command should be accepted");

    let mut riveter = None;
    let mut reedguard = None;
    for _ in 0..2_000 {
        world.step();
        riveter = ids_for(&world, 0, Kind::Riveter).first().copied();
        reedguard = ids_for(&world, 1, Kind::Reedguard).first().copied();
        if riveter.is_some() && reedguard.is_some() {
            break;
        }
    }
    let riveter = riveter.expect("Union unit was not produced");
    let reedguard = reedguard.expect("Assembly unit was not produced");
    let workers0 = ids_for(&world, 0, Kind::Hook);
    let workers1 = ids_for(&world, 1, Kind::Wick);
    world
        .issue(0, Command::Stop { units: workers0 })
        .expect("Union worker stop should be accepted");
    world
        .issue(1, Command::Stop { units: workers1 })
        .expect("Assembly worker stop should be accepted");
    world.step();
    world
        .issue(
            0,
            Command::Move {
                units: vec![riveter],
                target: Pos::cell(63, 64),
                queued: false,
            },
        )
        .expect("Union move to the gate landing should be accepted");
    world
        .issue(
            1,
            Command::Move {
                units: vec![reedguard],
                target: Pos::cell(65, 64),
                queued: false,
            },
        )
        .expect("Assembly move to the gate landing should be accepted");

    for _ in 0..4_000 {
        world.step();
        let at_landings = world
            .entities
            .iter()
            .any(|entity| entity.id == riveter && entity.pos == Pos::cell(63, 64))
            && world
                .entities
                .iter()
                .any(|entity| entity.id == reedguard && entity.pos == Pos::cell(65, 64));
        if at_landings {
            break;
        }
    }
    assert!(
        world
            .entities
            .iter()
            .any(|entity| entity.id == riveter && entity.pos == Pos::cell(63, 64))
    );
    assert!(
        world
            .entities
            .iter()
            .any(|entity| entity.id == reedguard && entity.pos == Pos::cell(65, 64))
    );
    world
        .issue(
            0,
            Command::Attack {
                units: vec![riveter],
                target: reedguard,
            },
        )
        .expect("Union attack command should be accepted in public gate vision");
    world
        .issue(
            1,
            Command::Attack {
                units: vec![reedguard],
                target: riveter,
            },
        )
        .expect("Assembly attack command should be accepted in public gate vision");

    let mut dead_id = None;
    let mut shot_events = 0usize;
    for _ in 0..9_000 {
        world.step();
        shot_events += world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Shot)
            .count();
        if let Some(event) = world
            .events
            .iter()
            .find(|event| event.kind == EventKind::Death)
        {
            dead_id = event.entity;
            break;
        }
    }
    let dead_id = dead_id.unwrap_or_else(|| {
        panic!(
            "the two real units never produced a Death event: shots={shot_events}, tick={}, {}, {}",
            world.tick,
            entity_state(&world, riveter),
            entity_state(&world, reedguard)
        )
    });
    assert!(!world.entities.iter().any(|entity| entity.id == dead_id));
    let (dead_owner, dead_kind, works) = if dead_id == riveter {
        (0u8, Kind::Riveter, works0)
    } else if dead_id == reedguard {
        (1u8, Kind::Reedguard, works1)
    } else {
        panic!("unexpected dead entity id {dead_id}");
    };

    let mut train_accepted = false;
    for attempt in 0..8_000 {
        if attempt % 30 == 0
            && world
                .issue(
                    dead_owner,
                    Command::Train {
                        building: works,
                        kind: dead_kind,
                    },
                )
                .is_ok()
        {
            train_accepted = true;
            break;
        }
        world.step();
    }
    assert!(
        train_accepted,
        "dead owner's production never accepted a replacement"
    );
    for _ in 0..2_000 {
        world.step();
        if world.entities.iter().any(|entity| {
            entity.owner == dead_owner && entity.kind == dead_kind && entity.id != dead_id
        }) {
            break;
        }
    }
    let replacement = world
        .entities
        .iter()
        .find(|entity| {
            entity.owner == dead_owner && entity.kind == dead_kind && entity.id != dead_id
        })
        .expect("replacement unit was not produced");
    assert!(
        replacement.id > dead_id,
        "replacement ID reused or moved backward"
    );
}

#[test]
fn malformed_nested_save_and_order_bounds_are_rejected() {
    let mut world = World::new(107, Faction::Union);
    world.ai_enabled = false;
    let valid_path = temp_path("valid");
    let malformed_path = temp_path("malformed");
    world.save(&valid_path).expect("valid save should succeed");
    let mut value: Value = serde_json::from_slice(&fs::read(&valid_path).expect("read valid save"))
        .expect("valid save is JSON");
    let production = json!({
        "kind": "Riveter",
        "remaining": 1,
        "started": false,
        "cost_salvage": 0,
        "cost_pressure": 0
    });
    value["world"]["entities"][0]["queue"] = Value::Array(vec![production; 17]);
    fs::write(
        &malformed_path,
        serde_json::to_vec(&value).expect("serialize malformed nested save"),
    )
    .expect("write malformed save");
    assert!(
        World::load(&malformed_path).is_err(),
        "oversized nested queue must reject"
    );

    let oversized_selection: Vec<u32> = (1..=129).collect();
    let order_error = world
        .issue(
            0,
            Command::Move {
                units: oversized_selection,
                target: Pos::cell(20, 20),
                queued: false,
            },
        )
        .expect_err("selection over the public bound must reject");
    assert!(order_error.contains("selection"));
    assert!(
        !world
            .command_log
            .last()
            .expect("rejection is logged")
            .accepted
    );
    let _ = fs::remove_file(valid_path);
    let _ = fs::remove_file(malformed_path);
}
