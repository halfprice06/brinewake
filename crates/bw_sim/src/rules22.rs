//! Rules 22: the trial 12 design changes.  A worker pulled back from a
//! wreck under fire hands its load in and goes back when the danger passes,
//! and DELIVER sends loaded workers to a yard; with three seats an enemy
//! Defense Nest at a mouth halves a count instead of stopping it; on the
//! Confluence a DRY ebbs back to the neutral tide after three minutes; the
//! hunters hunt: the Sounder's bonus covers a deployed Heliostat, both
//! hunters take three quarters of the damage from Loom shells, and the bonus gets the water
//! multiplier.
use super::*;

fn basin(factions: [Faction; 2]) -> World {
    let mut world =
        World::with_map(2200, MapId::SplitBasin, &factions).expect("two seats fit the Split Basin");
    world.ai_enabled = false;
    world
}

fn confluence() -> World {
    let mut world = World::with_map(
        2201,
        MapId::Confluence,
        &[Faction::Union, Faction::Assembly, Faction::Compact],
    )
    .expect("three seats fit the Confluence");
    world.ai_enabled = false;
    world
}

fn run(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.step();
    }
}

fn hq(world: &World, owner: u8) -> u32 {
    world
        .entities
        .iter()
        .find(|entity| entity.owner == owner && entity.kind == Kind::Headquarters)
        .map(|entity| entity.id)
        .expect("starting headquarters")
}

fn hold_everything(world: &mut World, seat: u8) {
    world.gate.owner = Some(seat);
    for arm in world.arms_of(seat) {
        let mouth = world.own_mouth(seat, arm).expect("own bank");
        world.spawn_for_tests(seat, Kind::Riveter, mouth);
    }
}

#[test]
fn rules_22_is_a_new_rules_version_with_its_own_digest() {
    assert_eq!(bw_content::RULES_VERSION, 22);
    assert_eq!(HUNTER_LOOM_SHELL_PERCENT, 75);
    assert_eq!(DRY_EBB_TICKS, 180 * 30);
    assert_eq!(bw_content::rules_digest(), bw_content::rules_digest());
}

// --- 1. Workers pulled back from danger ---------------------------------

/// A Union world with one wreck left for seat 0, and a starting Hook on it
/// carrying a load. Every other wreck is spent, so no safe wreck exists.
fn one_wreck_world() -> (World, u32, u32) {
    let mut world = basin([Faction::Union, Faction::Assembly]);
    let worker = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Hook)
        .map(|e| e.id)
        .expect("a starting Hook");
    let Order::Gather { resource } = world.entity(worker).expect("hook").order.clone() else {
        panic!("starting workers gather");
    };
    for item in &mut world.map.resources {
        if item.id != resource {
            item.remaining = 0;
        }
    }
    // The other starting workers stand aside, so only this one matters.
    for entity in &mut world.entities {
        if entity.owner == 0 && entity.kind == Kind::Hook && entity.id != worker {
            entity.order = Order::Idle;
        }
    }
    let item = world
        .map
        .resources
        .iter()
        .find(|item| item.id == resource)
        .cloned()
        .expect("wreck");
    let at = world.resource_work_target(&item, worker);
    let entity = world.entity_mut(worker).expect("hook");
    entity.pos = at;
    entity.carried = 10;
    entity.carried_kind = Some(ResourceKind::Salvage);
    (world, worker, resource)
}

/// The worker is hit at its wreck this tick: the danger pass runs on it.
fn hit(world: &mut World, worker: u32) {
    let pos = world.entity(worker).expect("worker").pos;
    world.events.push(Event {
        tick: world.tick,
        kind: EventKind::Damage,
        player: Some(1),
        entity: Some(worker),
        other: None,
        from: Some(pos),
        to: Some(pos),
        amount: 1,
        text: "damage".to_string(),
        cause: None,
    });
    let gatherers = world.gatherer_owners();
    world.update_worker_danger(&gatherers);
}

fn deposits(world: &World, worker: u32) -> u32 {
    world
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Deposit && e.entity == Some(worker))
        .map(|e| e.amount as u32)
        .sum()
}

#[test]
fn a_worker_pulled_back_hands_its_load_in_and_goes_back_when_the_mark_lapses() {
    let (mut world, worker, resource) = one_wreck_world();
    let base = hq(&world, 0);
    hit(&mut world, worker);
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Deliver {
            target: base,
            resource: Some(resource)
        },
        "no safe wreck: it falls back to the yard to deliver"
    );
    let until = world.players[0].worker_danger[0].until;
    // It walks home and hands its load in (trial 12: it never did).
    let mut delivered = 0;
    for _ in 0..40 * 30 {
        world.step();
        delivered += deposits(&world, worker);
        if delivered > 0 {
            break;
        }
    }
    assert_eq!(delivered, 10, "the load is handed in");
    assert_eq!(world.entity(worker).expect("hook").carried, 0);
    assert!(world.tick < until, "the mark still stands");
    // While the mark stands it waits at the yard, not at the wreck.
    let waiting_at = world.entity(worker).expect("hook").pos;
    run(&mut world, 30);
    let entity = world.entity(worker).expect("hook");
    assert!(matches!(entity.order, Order::Deliver { .. }), "it waits");
    assert!(entity.pos.distance_sq(waiting_at) <= i64::from(FP).pow(2));
    // The mark lapses: back to its own wreck.
    let left = until.saturating_sub(world.tick);
    run(&mut world, u32::try_from(left).expect("ticks") + 1);
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Gather { resource },
        "it resumes the wreck it was pulled off"
    );
}

#[test]
fn a_worker_pulled_back_with_no_wreck_left_ends_idle() {
    let (mut world, worker, resource) = one_wreck_world();
    hit(&mut world, worker);
    // Its wreck is emptied by someone else meanwhile.
    for item in &mut world.map.resources {
        if item.id == resource {
            item.remaining = 0;
        }
    }
    let mut delivered = 0;
    for _ in 0..40 * 30 {
        world.step();
        delivered += deposits(&world, worker);
        if world.entity(worker).expect("hook").order == Order::Idle {
            break;
        }
    }
    assert_eq!(delivered, 10, "it still hands its load in");
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Idle,
        "nothing to gather: truly idle, so the idle count finds it"
    );
}

#[test]
fn a_worker_pulled_back_takes_a_safe_wreck_when_one_appears() {
    let (mut world, worker, resource) = one_wreck_world();
    hit(&mut world, worker);
    // Walk it home and let it hand in.
    for _ in 0..40 * 30 {
        world.step();
        if world.entity(worker).expect("hook").carried == 0 {
            break;
        }
    }
    assert!(matches!(
        world.entity(worker).expect("hook").order,
        Order::Deliver { .. }
    ));
    // A wreck near the yard fills up again (flotsam, say): it goes there.
    let yard = world.entity(hq(&world, 0)).expect("hq").pos;
    let safe = world
        .map
        .resources
        .iter()
        .filter(|item| item.id != resource && item.kind == ResourceKind::Salvage)
        .filter(|item| world.auto_gatherable(Kind::Hook, item))
        .min_by_key(|item| item.pos.distance_sq(yard))
        .map(|item| item.id)
        .expect("another wreck");
    for item in &mut world.map.resources {
        if item.id == safe {
            item.remaining = 500;
        }
    }
    world.step();
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Gather { resource: safe }
    );
}

#[test]
fn deliver_sends_loaded_workers_to_the_yard_and_back_to_their_wreck() {
    let (mut world, worker, resource) = one_wreck_world();
    let base = hq(&world, 0);
    // A loaded worker the player had moved off, and a loaded gatherer.
    let other = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Hook && e.id != worker)
        .map(|e| e.id)
        .expect("a second Hook");
    let empty = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Hook && e.id != worker && e.id != other)
        .map(|e| e.id)
        .expect("a third Hook");
    {
        let entity = world.entity_mut(other).expect("hook");
        entity.carried = 7;
        entity.carried_kind = Some(ResourceKind::Salvage);
        entity.order = Order::Move { target: entity.pos };
    }
    world.entity_mut(empty).expect("hook").order = Order::Gather { resource };
    world
        .issue(
            0,
            Command::Deliver {
                units: vec![worker, other, empty],
                target: base,
            },
        )
        .expect("deliver at the headquarters");
    world.step();
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Deliver {
            target: base,
            resource: Some(resource)
        }
    );
    assert_eq!(
        world.entity(other).expect("hook").order,
        Order::Deliver {
            target: base,
            resource: None
        }
    );
    assert_eq!(
        world.entity(empty).expect("hook").order,
        Order::Gather { resource },
        "an empty gatherer keeps its wreck"
    );
    let mut delivered = 0;
    for _ in 0..40 * 30 {
        world.step();
        delivered += deposits(&world, worker) + deposits(&world, other);
        if delivered == 17 {
            break;
        }
    }
    assert_eq!(delivered, 17, "both loads handed in");
    world.step();
    assert_eq!(
        world.entity(worker).expect("hook").order,
        Order::Gather { resource },
        "back to the wreck it was on"
    );
    assert_eq!(
        world.entity(other).expect("hook").order,
        Order::Gather { resource },
        "with no wreck of its own, the nearest safe one"
    );
    // Only an own finished drop-off takes a delivery, and only workers do.
    let riveter = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(40, 60));
    assert!(
        world
            .issue(
                0,
                Command::Deliver {
                    units: vec![riveter],
                    target: base
                }
            )
            .is_err()
    );
    let enemy = hq(&world, 1);
    assert!(
        world
            .issue(
                0,
                Command::Deliver {
                    units: vec![worker],
                    target: enemy
                }
            )
            .is_err()
    );
    let works = world.spawn_for_tests(0, Kind::Works, Pos::cell(30, 40));
    assert!(
        world
            .issue(
                0,
                Command::Deliver {
                    units: vec![worker],
                    target: works
                }
            )
            .is_err(),
        "a Works is no drop-off"
    );
}

// --- 2. With three seats a nest slows a count ----------------------------

#[test]
fn with_three_seats_an_enemy_nest_at_a_mouth_halves_a_count() {
    let mut world = confluence();
    hold_everything(&mut world, 0);
    assert!(world.holds_every_lane(0));
    assert!(!world.hold_slowed(0));
    // Seat 1's nest on its own bank of the lane it shares with seat 0: the
    // trial 12 case.
    let arm = world.arm_between(0, 1).expect("neighbours share a lane");
    let far = world.own_mouth(1, arm).expect("seat 1's bank");
    let nest = world.spawn_for_tests(1, Kind::Tower, far);
    assert!(world.nests_slow_counts());
    assert!(world.mouth_slowed_for(0, far));
    assert!(!world.mouth_blocked_for(0, far), "it no longer blocks");
    assert!(world.holds_lane(0, arm), "the lane still counts");
    assert!(world.holds_every_lane(0));
    assert!(world.hold_slowed(0));
    world.lane_hold[0] = 0;
    run(&mut world, 60);
    assert_eq!(world.lane_hold[0], 30, "the count rises every other tick");
    // A nest still being dug does nothing.
    world.entity_mut(nest).expect("nest").build_remaining = 10;
    assert!(!world.hold_slowed(0));
    run(&mut world, 30);
    assert_eq!(world.lane_hold[0], 60);
    world.entity_mut(nest).expect("nest").build_remaining = 0;
    // A nest never slows its own side, and never holds.
    assert!(!world.mouth_slowed_for(1, far));
    // An enemy machine in the ring still stops the count, and it drains.
    let raider = world.spawn_for_tests(1, Kind::Riveter, far);
    assert!(!world.mouth_slowed_for(0, far), "a gun there blocks");
    assert!(world.mouth_blocked_for(0, far));
    assert!(!world.holds_every_lane(0));
    assert!(!world.hold_slowed(0));
    let before = world.lane_hold[0];
    world.step();
    assert_eq!(world.lane_hold[0], before - TIDE_HOLD_DRAIN_PER_TICK);
    world.entities.retain(|e| e.id != raider);
    // A flood still freezes it.
    world.gate.tide = Tide::Flood;
    world.gate.flood_until = Some(world.tick + 10 * 30);
    let before = world.lane_hold[0];
    run(&mut world, 30);
    assert_eq!(world.lane_hold[0], before, "frozen");
}

#[test]
fn with_three_seats_a_nest_elsewhere_does_not_slow_a_count() {
    let mut world = confluence();
    hold_everything(&mut world, 0);
    // The lane between seats 1 and 2 is none of seat 0's.
    let arm = world.arm_between(1, 2).expect("a lane");
    assert!(!world.arms_of(0).contains(&arm));
    let mouth = world.own_mouth(1, arm).expect("bank");
    world.spawn_for_tests(1, Kind::Tower, mouth);
    assert!(!world.hold_slowed(0));
    world.lane_hold[0] = 0;
    run(&mut world, 60);
    assert_eq!(world.lane_hold[0], 60);
}

#[test]
fn with_two_seats_a_nest_still_blocks_a_count() {
    let mut world = basin([Faction::Union, Faction::Assembly]);
    world.gate.owner = Some(0);
    for mouths in CROSSING_MOUTHS {
        world.spawn_for_tests(0, Kind::Riveter, mouths[0]);
    }
    assert!(world.holds_every_lane(0));
    let far = CROSSING_MOUTHS[0][1];
    world.spawn_for_tests(1, Kind::Tower, far);
    assert!(!world.nests_slow_counts());
    assert!(world.mouth_blocked_for(0, far));
    assert!(!world.mouth_slowed_for(0, far));
    assert!(!world.holds_every_lane(0));
    assert!(!world.hold_slowed(0));
    run(&mut world, 60);
    assert_eq!(world.lane_hold[0], 0, "the rules 20 block stands");
}

// --- 3. A DRY on the Confluence ebbs -------------------------------------

/// The owner dries `arm`; returns the tick it landed.
fn dry(world: &mut World, seat: u8, arm: Arm) -> u64 {
    world.gate.owner = Some(seat);
    world.players[seat as usize].pressure = 200;
    world
        .issue(seat, Command::SetTide { arm })
        .expect("the owner may dry an arm");
    for _ in 0..GATE_WARNING_TICKS + 60 {
        world.step();
        if world.gate.tide == Tide::Open && world.gate.dry_arm == arm {
            return world.tick - 1;
        }
    }
    panic!("the DRY never landed");
}

#[test]
fn a_dry_on_the_confluence_ebbs_back_to_neutral_after_three_minutes() {
    let mut world = confluence();
    let landed = dry(&mut world, 0, Arm(1));
    assert_eq!(world.gate.ebb_at, Some(landed + u64::from(DRY_EBB_TICKS)));
    // The warning starts ten seconds before, as a switch's does.
    let warn = landed + u64::from(DRY_EBB_TICKS) - u64::from(GATE_WARNING_TICKS);
    let wait = u32::try_from(warn - world.tick).expect("ticks");
    run(&mut world, wait);
    assert!(world.gate.warning_until.is_none(), "not yet");
    assert_eq!(world.gate.tide, Tide::Open);
    world.step();
    assert!(world.gate.ebb_pending);
    assert_eq!(
        world.gate.warning_until,
        Some(warn + u64::from(GATE_WARNING_TICKS))
    );
    assert!(
        world
            .events
            .iter()
            .any(|e| e.kind == EventKind::GateWarning && e.text == "the dry arm ebbs"),
        "heard as a switch warning"
    );
    // An enemy at the station does not cancel it: nobody paid for it.
    let (gx, gy) = world.map.gate_pos.cell_xy();
    let raider = world.spawn_for_tests(1, Kind::Riveter, Pos::cell(gx + 1, gy));
    // And the owner cannot switch over it.
    assert!(world.issue(0, Command::SetTide { arm: Arm(2) }).is_err());
    run(&mut world, GATE_WARNING_TICKS);
    world.entities.retain(|e| e.id != raider);
    assert_eq!(world.gate.tide, Tide::Neutral, "every arm shallow again");
    assert!(!world.gate.ebb_pending);
    assert_eq!(world.gate.ebb_at, None);
    assert!(!world.gate.opened);
    assert!(
        world
            .events
            .iter()
            .any(|e| e.kind == EventKind::GateChanged && e.text == "tide ebbs")
    );
    for arm in 0..3 {
        let centre = world.map.layout().arms[arm].centre;
        assert_eq!(
            world.depth_at(centre.0, centre.1),
            Some(Depth::Shallow),
            "arm {arm}"
        );
    }
}

#[test]
fn a_new_dry_before_the_ebb_replaces_it() {
    let mut world = confluence();
    dry(&mut world, 0, Arm(1));
    run(&mut world, 100 * 30);
    world.tick = world.tick.max(world.gate.locked_until);
    let landed = dry(&mut world, 0, Arm(2));
    assert_eq!(world.gate.ebb_at, Some(landed + u64::from(DRY_EBB_TICKS)));
    run(&mut world, 170 * 30);
    assert_eq!(world.gate.tide, Tide::Open, "the new DRY has its own 180 s");
    assert_eq!(world.gate.dry_arm, Arm(2));
}

#[test]
fn a_flood_over_a_dry_falls_to_neutral_if_the_ebb_came_due_under_it() {
    let mut world = confluence();
    dry(&mut world, 0, Arm(1));
    // A flood late in the DRY (before the ebb's own warning at 170 s):
    // the ebb comes due while it stands.
    run(&mut world, 160 * 30);
    world.players[0].pressure = 200;
    world.issue(0, Command::Flood).expect("flood");
    run(&mut world, FLOOD_WARNING_TICKS + 1);
    assert_eq!(world.gate.tide, Tide::Flood);
    run(&mut world, FLOOD_TICKS);
    assert_eq!(world.gate.tide, Tide::Neutral);
    assert_eq!(world.gate.ebb_at, None);

    // A flood early in the DRY falls back to it, and the ebb keeps time.
    let mut world = confluence();
    dry(&mut world, 0, Arm(1));
    let ebb = world.gate.ebb_at.expect("ebb due");
    world.tick = world.tick.max(world.gate.locked_until);
    world.players[0].pressure = 200;
    world.issue(0, Command::Flood).expect("flood");
    run(&mut world, FLOOD_WARNING_TICKS + FLOOD_TICKS + 1);
    assert_eq!(world.gate.tide, Tide::Open);
    assert_eq!(world.gate.ebb_at, Some(ebb));
    let wait = u32::try_from(ebb - world.tick).expect("ticks");
    run(&mut world, wait + 1);
    assert_eq!(world.gate.tide, Tide::Neutral);
}

#[test]
fn a_dry_on_the_split_basin_never_ebbs() {
    let mut world = basin([Faction::Union, Faction::Assembly]);
    assert!(!world.dry_ebbs());
    dry(&mut world, 0, Arm::SOUTH);
    assert_eq!(world.gate.ebb_at, None);
    run(&mut world, 200 * 30);
    assert_eq!(world.gate.tide, Tide::Open);
    assert!(world.gate.warning_until.is_none());
}

// --- 4. Hunters that hunt -------------------------------------------------

/// A tidal lane cell of arm 0, shallow on the neutral tide.
fn wading_cell(world: &World) -> Pos {
    for y in 0..i32::from(world.map.height) {
        for x in 0..i32::from(world.map.width) {
            if world.map.terrain(x, y) == Terrain::Lane0
                && world.depth_at(x, y) == Some(Depth::Shallow)
            {
                return Pos::cell(x, y);
            }
        }
    }
    panic!("no shallow lane cell");
}

#[test]
fn the_sounder_hunts_a_deployed_heliostat_and_the_bonus_wades_too() {
    let mut world = basin([Faction::Union, Faction::Compact]);
    let sounder = world.spawn_for_tests(0, Kind::Sounder, Pos::cell(40, 60));
    let glinter = world.spawn_for_tests(1, Kind::Glinter, Pos::cell(40, 62));
    let helio = world.spawn_for_tests(1, Kind::Heliostat, Pos::cell(42, 60));
    let shot = spec(Kind::Sounder).damage;
    assert_eq!(world.modified_damage(sounder, helio, shot), shot, "packed");
    world.entity_mut(helio).expect("helio").deployed = true;
    assert_eq!(
        world.modified_damage(sounder, helio, shot),
        shot + SOUNDER_LOOM_BONUS
    );
    // The Glinter's bonus stays the Loom's alone.
    let loom = world.spawn_for_tests(0, Kind::Loom, Pos::cell(42, 62));
    world.entity_mut(loom).expect("loom").deployed = true;
    let flash = spec(Kind::Glinter).damage;
    assert_eq!(
        world.modified_damage(glinter, loom, flash),
        flash + GLINTER_LOOM_BONUS
    );
    // The hunter bonus now takes the water share with the rest of the
    // shot: a wading deployed Loom takes (7 + 5) x 125%.
    let wet = wading_cell(&world);
    let mut basin_b = basin([Faction::Union, Faction::Assembly]);
    let hunter = basin_b.spawn_for_tests(0, Kind::Sounder, Pos::cell(40, 60));
    let wading = basin_b.spawn_for_tests(1, Kind::Loom, wet);
    basin_b.entity_mut(wading).expect("loom").deployed = true;
    let entity = basin_b.entity(wading).expect("loom").clone();
    assert!(basin_b.wading(&entity));
    assert_eq!(
        basin_b.modified_damage(hunter, wading, shot),
        (shot + SOUNDER_LOOM_BONUS) * WADING_DAMAGE_PERCENT / 100
    );
    assert_eq!(basin_b.modified_damage(hunter, wading, shot), 15);
    // Swamped: x 150%.
    basin_b.gate.tide = Tide::Flood;
    let entity = basin_b.entity(wading).expect("loom").clone();
    assert!(basin_b.swamped(&entity));
    assert_eq!(basin_b.modified_damage(hunter, wading, shot), 18);
}

#[test]
fn a_loom_shell_does_three_quarters_damage_to_the_hunters() {
    let mut world = basin([Faction::Union, Faction::Assembly]);
    let loom = world.spawn_for_tests(1, Kind::Loom, Pos::cell(50, 60));
    let shot = ArtilleryShot {
        owner: 1,
        source: loom,
        from: Pos::cell(50, 60),
        target: Pos::cell(44, 60),
        impact_tick: 0,
    };
    let full = spec(Kind::Loom).damage;
    for (kind, expected) in [
        (Kind::Sounder, full * HUNTER_LOOM_SHELL_PERCENT / 100),
        (Kind::Glinter, full * HUNTER_LOOM_SHELL_PERCENT / 100),
        (Kind::Riveter, full),
        (Kind::Brander, full),
    ] {
        let id = world.spawn_for_tests(0, kind, Pos::cell(44, 60));
        let target = world.entity(id).expect("target").clone();
        assert_eq!(world.artillery_damage(&shot, &target), expected, "{kind:?}");
        world.entities.retain(|e| e.id != id);
    }
}

// --- Duels: does the named answer beat the unit? -------------------------

/// The result of a duel: for each side, machines left and hull left.
#[derive(Debug, Clone, Copy)]
struct Duel {
    attackers_left: usize,
    attacker_hull: i32,
    defenders_left: usize,
    defender_hull: i32,
    seconds: u64,
}

/// Open dry ground `w` by `h` cells, clear of every building by 16 cells.
fn open_ground(world: &World, w: i32, h: i32) -> (i32, i32) {
    let clear = i64::from(FP * 16).pow(2);
    for y0 in 4..i32::from(world.map.height) - h - 4 {
        for x0 in 4..i32::from(world.map.width) - w - 4 {
            let dry = (y0..y0 + h).all(|y| {
                (x0..x0 + w)
                    .all(|x| matches!(world.map.terrain(x, y), Terrain::Salt | Terrain::Silt))
            });
            if !dry {
                continue;
            }
            let centre = Pos::cell(x0 + w / 2, y0 + h / 2);
            let far = world.entities.iter().all(|e| {
                !e.kind.is_building()
                    || e.pos.distance_sq(centre) > clear + i64::from(FP * w).pow(2)
            });
            if far {
                return (x0, y0);
            }
        }
    }
    panic!("no open ground");
}

/// `attackers` of seat 0 attack-move through `defenders` of seat 1, who
/// stand in a line, deployed if `deployed`, on open dry ground. Every
/// other machine is cleared away.
fn duel(
    factions: [Faction; 2],
    attackers: (Kind, usize),
    defenders: (Kind, usize),
    deployed: bool,
) -> Duel {
    let mut world = basin(factions);
    world.entities.retain(|e| e.kind.is_building());
    let (x0, y0) = open_ground(&world, 34, 16);
    let mid = y0 + 8;
    let line_x = x0 + 26;
    let mut defs = Vec::new();
    for i in 0..defenders.1 {
        let y = mid - (defenders.1 as i32) / 2 + i as i32;
        let id = world.spawn_for_tests(1, defenders.0, Pos::cell(line_x, y));
        if deployed {
            let e = world.entity_mut(id).expect("defender");
            e.deployed = true;
            e.deploy_remaining = 0;
            e.facing = 6;
        }
        defs.push(id);
    }
    let mut atts = Vec::new();
    for i in 0..attackers.1 {
        let col = (i % 2) as i32;
        let row = (i / 2) as i32;
        let y = mid - (attackers.1 as i32 / 2) / 2 + row;
        atts.push(world.spawn_for_tests(0, attackers.0, Pos::cell(x0 + 2 + col, y)));
    }
    world
        .issue(
            0,
            Command::AttackMove {
                units: atts.clone(),
                target: Pos::cell(line_x + 3, mid),
                queued: false,
            },
        )
        .expect("attack-move");
    let alive = |world: &World, ids: &[u32]| -> (usize, i32) {
        ids.iter()
            .filter_map(|id| world.entity(*id))
            .fold((0, 0), |(n, hull), e| (n + 1, hull + e.hp))
    };
    let start = world.tick;
    for _ in 0..150 * 30 {
        world.step();
        if alive(&world, &atts).0 == 0 || alive(&world, &defs).0 == 0 {
            break;
        }
        // Survivors that reached the far side turn back into the fight.
        if (world.tick - start).is_multiple_of(10 * 30) {
            let left: Vec<u32> = atts
                .iter()
                .copied()
                .filter(|id| world.entity(*id).is_some())
                .collect();
            let target = defs
                .iter()
                .find_map(|id| world.entity(*id).map(|e| e.pos))
                .unwrap_or(Pos::cell(line_x, mid));
            let _ = world.issue(
                0,
                Command::AttackMove {
                    units: left,
                    target,
                    queued: false,
                },
            );
        }
    }
    let (attackers_left, attacker_hull) = alive(&world, &atts);
    let (defenders_left, defender_hull) = alive(&world, &defs);
    Duel {
        attackers_left,
        attacker_hull,
        defenders_left,
        defender_hull,
        seconds: (world.tick - start) / 30,
    }
}

#[test]
fn sounders_beat_deployed_heliostats_for_equal_salvage() {
    // 15 Sounders (1,200) against 8 deployed Heliostats (1,200).
    let result = duel(
        [Faction::Union, Faction::Compact],
        (Kind::Sounder, 15),
        (Kind::Heliostat, 8),
        true,
    );
    assert_eq!(result.defenders_left, 0, "{result:?}");
    assert!(result.attackers_left > 0, "{result:?}");
    assert!(
        result.attacker_hull > 0 && result.defender_hull == 0 && result.seconds < 150,
        "{result:?}"
    );
}

#[test]
fn glinters_beat_deployed_looms_for_equal_salvage() {
    // 16 Glinters (1,280) against 9 deployed Looms (1,305).
    let result = duel(
        [Faction::Compact, Faction::Assembly],
        (Kind::Glinter, 16),
        (Kind::Loom, 9),
        true,
    );
    assert_eq!(result.defenders_left, 0, "{result:?}");
    assert!(result.attackers_left > 0, "{result:?}");
    assert!(
        result.attacker_hull > 0 && result.defender_hull == 0 && result.seconds < 150,
        "{result:?}"
    );
}

#[test]
fn sounders_beat_deployed_looms_for_equal_salvage() {
    // 16 Sounders (1,280) against 9 deployed Looms (1,305).
    let result = duel(
        [Faction::Union, Faction::Assembly],
        (Kind::Sounder, 16),
        (Kind::Loom, 9),
        true,
    );
    assert_eq!(result.defenders_left, 0, "{result:?}");
    assert!(result.attackers_left > 0, "{result:?}");
    assert!(
        result.attacker_hull > 0 && result.defender_hull == 0 && result.seconds < 150,
        "{result:?}"
    );
}
