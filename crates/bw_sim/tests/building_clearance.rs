use bw_core::{FP, Faction, Kind, Pos};
use bw_sim::{Command, EventKind, Order, World};
use std::collections::BTreeMap;

fn ids_for(world: &World, owner: u8, kind: Kind) -> Vec<u32> {
    world
        .entities
        .iter()
        .filter(|entity| entity.owner == owner && entity.kind == kind && entity.hp > 0)
        .map(|entity| entity.id)
        .collect()
}

fn building_footprint(kind: Kind) -> i32 {
    match kind {
        Kind::Headquarters => 4,
        Kind::Works => 3,
        Kind::Dropoff | Kind::Condenser | Kind::Tower => 2,
        _ => 1,
    }
}

fn positions(world: &World) -> BTreeMap<u32, Pos> {
    world
        .entities
        .iter()
        .map(|entity| (entity.id, entity.pos))
        .collect()
}

fn point_in_building(pos: Pos, building: &bw_sim::Entity) -> bool {
    let (x, y) = building.pos.cell_xy();
    let size = building_footprint(building.kind);
    let min_x = x * FP + 1;
    let max_x = (x + size) * FP - 1;
    let min_y = y * FP + 1;
    let max_y = (y + size) * FP - 1;
    pos.x >= min_x && pos.x < max_x && pos.y >= min_y && pos.y < max_y
}

fn segment_enters_building(from: Pos, to: Pos, building: &bw_sim::Entity) -> bool {
    // Liang-Barsky clipping against the open interior of the authoritative
    // footprint catches a unit crossing the rectangle between two ticks.
    let (x, y) = building.pos.cell_xy();
    let size = building_footprint(building.kind);
    let min_x = f64::from(x * FP + 1);
    let max_x = f64::from((x + size) * FP - 1);
    let min_y = f64::from(y * FP + 1);
    let max_y = f64::from((y + size) * FP - 1);
    let ax = f64::from(from.x);
    let ay = f64::from(from.y);
    let dx = f64::from(to.x - from.x);
    let dy = f64::from(to.y - from.y);
    let mut enter: f64 = 0.0;
    let mut leave: f64 = 1.0;
    for (p, q) in [
        (-dx, ax - min_x),
        (dx, max_x - ax),
        (-dy, ay - min_y),
        (dy, max_y - ay),
    ] {
        if p == 0.0 {
            if q <= 0.0 {
                return false;
            }
            continue;
        }
        let ratio = q / p;
        if p < 0.0 {
            enter = enter.max(ratio);
        } else {
            leave = leave.min(ratio);
        }
        if enter >= leave {
            return false;
        }
    }
    enter < 1.0 && leave > 0.0 && enter < leave
}

fn assert_mobile_clear(world: &World, previous: &mut BTreeMap<u32, Pos>, context: &str) {
    let buildings: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| entity.hp > 0 && entity.kind.is_building())
        .collect();
    for entity in world
        .entities
        .iter()
        .filter(|entity| entity.hp > 0 && !entity.kind.is_building())
    {
        for building in &buildings {
            assert!(
                !point_in_building(entity.pos, building),
                "{context}: entity {} {:?} raw=({}, {}) entered {:?} footprint at cell {:?}",
                entity.id,
                entity.kind,
                entity.pos.x,
                entity.pos.y,
                building.kind,
                building.pos.cell_xy()
            );
            if let Some(from) = previous.get(&entity.id) {
                assert!(
                    !segment_enters_building(*from, entity.pos, building),
                    "{context}: entity {} {:?} crossed {:?} footprint from {:?} to {:?}",
                    entity.id,
                    entity.kind,
                    building.kind,
                    from,
                    entity.pos
                );
            }
        }
        previous.insert(entity.id, entity.pos);
    }
}

fn step_checked(world: &mut World, previous: &mut BTreeMap<u32, Pos>, context: &str) {
    world.step();
    assert_mobile_clear(world, previous, context);
}

fn finish_works(
    world: &mut World,
    previous: &mut BTreeMap<u32, Pos>,
    worker: u32,
    pos: Pos,
    context: &str,
) -> u32 {
    world
        .issue(
            0,
            Command::Build {
                worker,
                kind: Kind::Works,
                pos,
                queued: false,
            },
        )
        .expect("Works placement should be accepted");
    let works = loop {
        step_checked(world, previous, context);
        if let Some(entity) = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Works && entity.pos == pos)
        {
            break entity.id;
        }
        assert!(world.tick < 2_000, "Works was never spawned");
    };
    for _ in 0..2_000 {
        if world
            .entities
            .iter()
            .any(|entity| entity.id == works && entity.build_remaining == 0)
        {
            return works;
        }
        step_checked(world, previous, context);
    }
    panic!("Works {works} did not complete construction");
}

#[test]
fn gather_return_stays_clear_of_headquarters_in_raw_motion() {
    let mut world = World::new(4_101, Faction::Union);
    world.ai_enabled = false;
    let worker = ids_for(&world, 0, Kind::Hook)[0];
    let resource = world
        .map
        .resources
        .iter()
        .find(|resource| resource.pos.cell_xy() == (16, 57))
        .expect("left starting salvage bed")
        .id;
    let mut previous = positions(&world);
    world
        .issue(
            0,
            Command::Gather {
                units: vec![worker],
                resource,
            },
        )
        .expect("gather command should be accepted");

    let mut deposits = 0;
    for _ in 0..1_800 {
        step_checked(&mut world, &mut previous, "normal gather return");
        deposits += world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Deposit && event.entity == Some(worker))
            .count();
        if deposits > 0 {
            break;
        }
    }
    assert!(deposits > 0, "worker never returned a gathered load");
}

#[test]
fn build_repair_and_move_around_hq_and_works_stay_clear() {
    let mut world = World::new(4_102, Faction::Union);
    world.ai_enabled = false;
    let workers = ids_for(&world, 0, Kind::Hook);
    let worker = workers[0];
    let mut previous = positions(&world);
    let works = finish_works(
        &mut world,
        &mut previous,
        worker,
        Pos::cell(25, 60),
        "build Works",
    );

    let max_hp = world
        .entities
        .iter()
        .find(|entity| entity.id == works)
        .expect("finished Works")
        .max_hp;
    world
        .entities
        .iter_mut()
        .find(|entity| entity.id == works)
        .expect("finished Works")
        .hp = max_hp - 100;
    world
        .issue(
            0,
            Command::Repair {
                units: vec![worker],
                target: works,
            },
        )
        .expect("repair command should be accepted");
    for _ in 0..240 {
        step_checked(&mut world, &mut previous, "repair Works");
        if world
            .entities
            .iter()
            .find(|entity| entity.id == works)
            .is_some_and(|entity| entity.hp == entity.max_hp)
        {
            break;
        }
    }
    assert_eq!(
        world
            .entities
            .iter()
            .find(|entity| entity.id == works)
            .expect("Works remains alive")
            .hp,
        max_hp,
        "repair worker did not reach its interaction slot"
    );

    // The same worker crosses the yard toward the far side of the starting
    // headquarters, forcing cardinal pathing around both building footprints.
    world
        .issue(
            0,
            Command::Move {
                units: vec![worker],
                target: Pos::cell(7, 66),
                queued: false,
            },
        )
        .expect("yard move should be accepted");
    for _ in 0..1_800 {
        step_checked(&mut world, &mut previous, "move around HQ and Works");
        if world
            .entities
            .iter()
            .find(|entity| entity.id == worker)
            .is_some_and(|entity| entity.pos == Pos::cell(7, 66))
        {
            break;
        }
    }
    assert_eq!(
        world
            .entities
            .iter()
            .find(|entity| entity.id == worker)
            .expect("worker remains alive")
            .pos,
        Pos::cell(7, 66),
        "worker did not complete the route around HQ and Works"
    );
}

#[test]
fn production_spawn_and_rally_start_outside_works_footprint() {
    let mut world = World::new(4_103, Faction::Union);
    world.ai_enabled = false;
    let worker = ids_for(&world, 0, Kind::Hook)[0];
    let mut previous = positions(&world);
    let works = finish_works(
        &mut world,
        &mut previous,
        worker,
        Pos::cell(25, 60),
        "spawn Works",
    );
    world
        .issue(
            0,
            Command::Rally {
                building: works,
                pos: Pos::cell(35, 60),
            },
        )
        .expect("Works rally should be accepted");
    world
        .issue(
            0,
            Command::Train {
                building: works,
                kind: Kind::Riveter,
            },
        )
        .expect("Riveter train should be accepted");

    let mut spawned = None;
    for _ in 0..1_200 {
        step_checked(&mut world, &mut previous, "Works production spawn");
        if let Some(event) = world
            .events
            .iter()
            .find(|event| event.kind == EventKind::ProductionCompleted)
        {
            spawned = event.entity;
            break;
        }
    }
    let spawned = spawned.expect("Works did not spawn a Riveter");
    let unit = world
        .entities
        .iter()
        .find(|entity| entity.id == spawned)
        .expect("spawned Riveter remains alive");
    assert_eq!(unit.kind, Kind::Riveter);
    assert_eq!(
        unit.order,
        Order::Move {
            target: Pos::cell(35, 60)
        }
    );
}

#[test]
fn blocked_interaction_slot_keeps_repairer_outside_works() {
    let mut world = World::new(4_104, Faction::Union);
    world.ai_enabled = false;
    let workers = ids_for(&world, 0, Kind::Hook);
    let builder = workers[0];
    let repairer = workers[1];
    let mut previous = positions(&world);
    let works = finish_works(
        &mut world,
        &mut previous,
        builder,
        Pos::cell(25, 60),
        "blocked Works",
    );

    // Pin the repair request to one perimeter slot, then occupy that slot
    // with another worker. This models a slot becoming blocked after the
    // worker has selected it and lets the test observe the navigation wait.
    let works_pos = world
        .entities
        .iter()
        .find(|entity| entity.id == works)
        .expect("Works remains alive")
        .pos
        .cell_xy();
    let (x, y) = works_pos;
    let blocked_slot = Pos::cell(x - 1, y - 1);
    let blocker = workers[2];
    if let Some(entity) = world
        .entities
        .iter_mut()
        .find(|entity| entity.id == blocker)
    {
        entity.pos = blocked_slot;
        entity.order = Order::Idle;
        entity.path.clear();
        entity.path_index = 0;
        entity.path_target = None;
    }

    let max_hp = world
        .entities
        .iter()
        .find(|entity| entity.id == works)
        .expect("Works remains alive")
        .max_hp;
    world
        .entities
        .iter_mut()
        .find(|entity| entity.id == works)
        .expect("Works remains alive")
        .hp = max_hp - 100;
    if let Some(entity) = world
        .entities
        .iter_mut()
        .find(|entity| entity.id == repairer)
    {
        entity.pos = Pos::cell(20, 50);
        entity.order = Order::Idle;
        entity.path.clear();
        entity.path_index = 0;
        // Preserve the selected slot through command application. The
        // Repair command clears the path but intentionally leaves a pinned
        // interaction target in place.
        entity.path_target = Some(blocked_slot);
    }
    previous = positions(&world);
    world
        .issue(
            0,
            Command::Repair {
                units: vec![repairer],
                target: works,
            },
        )
        .expect("repair command should be accepted despite blocked slots");
    let mut saw_blocked = false;
    for _ in 0..240 {
        step_checked(&mut world, &mut previous, "blocked Works slots");
        saw_blocked |= world
            .entities
            .iter()
            .find(|entity| entity.id == repairer)
            .is_some_and(|entity| entity.blocked_ticks > 0);
    }
    let repairer = world
        .entities
        .iter()
        .find(|entity| entity.id == repairer)
        .expect("repair worker remains alive");
    assert!(
        saw_blocked,
        "repairer never waited on occupied slot: pos={:?} target={:?} path={}/{} order={:?}",
        repairer.pos.cell_xy(),
        repairer.path_target.map(|target| target.cell_xy()),
        repairer.path_index,
        repairer.path.len(),
        repairer.order
    );
    assert_ne!(
        repairer.pos, blocked_slot,
        "repairer entered the occupied slot"
    );
}

#[test]
fn building_placed_ahead_of_moving_unit_stops_at_footprint_boundary() {
    let mut world = World::new(4_105, Faction::Union);
    world.ai_enabled = false;
    let workers = ids_for(&world, 0, Kind::Hook);
    let moving = workers[0];
    let builder = workers[1];
    if let Some(entity) = world.entities.iter_mut().find(|entity| entity.id == moving) {
        entity.pos = Pos::cell(20, 61);
        entity.order = Order::Idle;
        entity.path.clear();
        entity.path_index = 0;
        entity.path_target = None;
    }
    let mut previous = positions(&world);
    world
        .issue(
            0,
            Command::Move {
                units: vec![moving],
                target: Pos::cell(40, 61),
                queued: false,
            },
        )
        .expect("moving command should be accepted");
    for _ in 0..12 {
        step_checked(&mut world, &mut previous, "moving before placement");
    }
    world
        .issue(
            0,
            Command::Build {
                worker: builder,
                kind: Kind::Works,
                pos: Pos::cell(25, 60),
                queued: false,
            },
        )
        .expect("building placement ahead of the unit should be accepted");

    let mut works = None;
    for _ in 0..1_200 {
        step_checked(&mut world, &mut previous, "building placed during movement");
        works = world
            .entities
            .iter()
            .find(|entity| entity.kind == Kind::Works && entity.pos == Pos::cell(25, 60))
            .map(|entity| entity.id);
        if world
            .entities
            .iter()
            .find(|entity| entity.id == moving)
            .is_some_and(|entity| entity.blocked_ticks > 0)
        {
            break;
        }
    }
    assert!(works.is_some(), "the planned Works was never placed");
    let moving = world
        .entities
        .iter()
        .find(|entity| entity.id == moving)
        .expect("moving worker remains alive");
    assert!(
        moving.blocked_ticks > 0,
        "moving worker did not stop at Works"
    );
    assert_eq!(
        moving.pos.cell_xy().0,
        24,
        "moving worker should wait in the cell west of Works"
    );
}
