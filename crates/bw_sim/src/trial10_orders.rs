//! Rules 19: orders from the tenth trial.  A shift-queued gather or move
//! waits for the site a worker is on; CAPTURE and an out-of-reach ATTACK
//! pack a deployed machine first; a gun sent at one target deploys when it
//! has it in reach; and the interface can ask whether a move is possible.
use super::*;

fn fixture(faction: Faction) -> World {
    let mut world = World::new(1400, faction);
    world.ai_enabled = false;
    world.reset_fixture_origin().expect("clean fixture origin");
    world
}

fn unit(world: &mut World, owner: u8, kind: Kind, cell: (i32, i32)) -> u32 {
    world.spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1))
}

fn run(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.step();
    }
}

fn site_near(world: &World, kind: Kind, cell: (i32, i32)) -> Pos {
    for radius in 0..12 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let pos = Pos::cell(cell.0 + dx, cell.1 + dy);
                if world.can_place(kind, pos) {
                    return pos;
                }
            }
        }
    }
    panic!("no site near {cell:?}");
}

fn wreck(world: &World) -> u32 {
    world
        .map
        .resources
        .iter()
        .filter(|r| r.remaining > 0 && r.kind == ResourceKind::Salvage)
        .min_by_key(|r| (r.pos.distance_sq(Pos::cell(24, 58)), r.id))
        .map(|r| r.id)
        .expect("a wreck")
}

fn site_at(world: &World, kind: Kind, pos: Pos) -> u32 {
    world
        .entities
        .iter()
        .find(|e| e.kind == kind && e.pos == pos)
        .map(|e| e.id)
        .expect("site")
}

fn build_palisade(world: &mut World, worker: u32) -> u32 {
    world.players[0].salvage = 2000;
    let pos = site_near(world, Kind::Palisade, (26, 58));
    world
        .issue(
            0,
            Command::Build {
                worker,
                kind: Kind::Palisade,
                pos,
                queued: false,
            },
        )
        .expect("site");
    world.step();
    site_at(world, Kind::Palisade, pos)
}

fn finish(world: &mut World, site: u32) {
    for _ in 0..3000 {
        world.step();
        if world.entity(site).is_some_and(|e| e.build_remaining == 0) {
            break;
        }
    }
    assert_eq!(world.entity(site).expect("site").build_remaining, 0);
    world.step();
}

#[test]
fn a_shift_gather_waits_for_the_site_the_worker_is_building() {
    let mut world = fixture(Faction::Union);
    let worker = unit(&mut world, 0, Kind::Hook, (24, 58));
    let site = build_palisade(&mut world, worker);
    let resource = wreck(&world);
    world
        .issue(
            0,
            Command::QueueGather {
                units: vec![worker],
                resource,
            },
        )
        .expect("queued gather");
    world.step();
    // Trial 10: the gather replaced the build and the site was left with
    // no builder.
    assert_eq!(world.site_builder(site), SiteBuilder::Building);
    assert_eq!(
        world.entity(worker).map(|e| e.order.clone()),
        Some(Order::Build { target: site })
    );
    finish(&mut world, site);
    assert_eq!(
        world.entity(worker).map(|e| e.order.clone()),
        Some(Order::Gather { resource }),
        "the worker gathers once the site stands"
    );
    // A worker with no site gathers at once.
    let idle = unit(&mut world, 0, Kind::Hook, (22, 60));
    world
        .issue(
            0,
            Command::QueueGather {
                units: vec![idle],
                resource,
            },
        )
        .expect("queued gather");
    world.step();
    assert_eq!(
        world.entity(idle).map(|e| e.order.clone()),
        Some(Order::Gather { resource })
    );
}

#[test]
fn a_plain_order_drops_what_a_builder_was_to_do_after() {
    let mut world = fixture(Faction::Union);
    let worker = unit(&mut world, 0, Kind::Hook, (24, 58));
    let site = build_palisade(&mut world, worker);
    let resource = wreck(&world);
    world
        .issue(
            0,
            Command::QueueGather {
                units: vec![worker],
                resource,
            },
        )
        .expect("queued gather");
    world.step();
    world
        .issue(
            0,
            Command::Repair {
                units: vec![worker],
                target: site,
            },
        )
        .expect("resume");
    world.step();
    assert!(world.entity(worker).expect("worker").after_build.is_none());
    finish(&mut world, site);
    assert_eq!(
        world.entity(worker).map(|e| e.order.clone()),
        Some(Order::Idle)
    );
}

#[test]
fn a_shift_move_waits_for_the_site_too() {
    let mut world = fixture(Faction::Union);
    let worker = unit(&mut world, 0, Kind::Hook, (24, 58));
    let site = build_palisade(&mut world, worker);
    let target = Pos::cell(20, 60);
    world
        .issue(
            0,
            Command::Move {
                units: vec![worker],
                target,
                queued: true,
            },
        )
        .expect("queued move");
    world.step();
    assert_eq!(world.site_builder(site), SiteBuilder::Building);
    finish(&mut world, site);
    assert!(
        matches!(
            world.entity(worker).map(|e| e.order.clone()),
            Some(Order::Move { .. })
        ),
        "{:?}",
        world.entity(worker).map(|e| e.order.clone())
    );
}

#[test]
fn capture_packs_a_deployed_heliostat_first_and_then_goes() {
    let mut world = fixture(Faction::Compact);
    let heliostat = unit(&mut world, 0, Kind::Heliostat, (40, 64));
    world
        .issue(
            0,
            Command::SetDeployed {
                units: vec![heliostat],
                deployed: true,
            },
        )
        .expect("deploy");
    run(&mut world, 120);
    assert!(world.entity(heliostat).expect("heliostat").deployed);
    let start = world.entity(heliostat).expect("heliostat").pos;
    world
        .issue(
            0,
            Command::Capture {
                units: vec![heliostat],
            },
        )
        .expect("capture");
    world.step();
    let packing = world.entity(heliostat).expect("heliostat");
    assert!(packing.deploy_remaining > 0 && !packing.deploy_target);
    assert_eq!(packing.after_pack, Some(Order::Capture));
    run(&mut world, 240);
    let going = world.entity(heliostat).expect("heliostat");
    assert!(!going.deployed);
    assert_eq!(going.order, Order::Capture);
    assert_ne!(going.pos, start, "trial 10: nothing moved");
}

#[test]
fn a_kept_deployed_gun_stays_on_a_capture() {
    let mut world = fixture(Faction::Compact);
    let heliostat = unit(&mut world, 0, Kind::Heliostat, (40, 64));
    for command in [
        Command::SetDeployed {
            units: vec![heliostat],
            deployed: true,
        },
        Command::KeepDeployed {
            units: vec![heliostat],
            keep: true,
        },
    ] {
        world.issue(0, command).expect("order");
    }
    run(&mut world, 120);
    world
        .issue(
            0,
            Command::Capture {
                units: vec![heliostat],
            },
        )
        .expect("capture");
    run(&mut world, 30);
    let kept = world.entity(heliostat).expect("heliostat");
    assert!(kept.deployed);
    assert_eq!(kept.order, Order::Deploy, "it keeps firing where it stands");
}

/// A walkable cell `distance` cells east of `from`, or further, that a Loom
/// can walk to.
fn reachable_east(world: &World, from: (i32, i32), distance: i32) -> (i32, i32) {
    for dx in distance..distance + 12 {
        for dy in [0, 1, -1, 2, -2, 3, -3] {
            let cell = (from.0 + dx, from.1 + dy);
            if world.cell_walkable_for(Kind::Loom, cell)
                && !world.find_path(from, cell, Kind::Loom).is_empty()
            {
                return cell;
            }
        }
    }
    panic!("no reachable cell east of {from:?}");
}

#[test]
fn a_deployed_loom_sent_at_a_target_out_of_reach_packs_goes_and_deploys() {
    let mut world = fixture(Faction::Assembly);
    let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
    world
        .issue(
            0,
            Command::SetDeployed {
                units: vec![loom],
                deployed: true,
            },
        )
        .expect("deploy");
    run(&mut world, 60);
    let range = world.weapon_range(world.entity(loom).expect("loom")) / FP;
    let far = reachable_east(&world, (40, 64), range + 5);
    let enemy = unit(&mut world, 1, Kind::Hook, far);
    if let Some(e) = world.entity_mut(enemy) {
        e.order = Order::Hold;
    }
    // An unarmed scout of ours lights the target.
    unit(&mut world, 0, Kind::Tidewatch, (far.0 - 2, far.1));
    world.step();
    assert!(world.entity_visible(0, enemy));
    world
        .issue(
            0,
            Command::Attack {
                units: vec![loom],
                target: enemy,
            },
        )
        .expect("attack");
    world.step();
    let packing = world.entity(loom).expect("loom");
    assert!(
        packing.deploy_remaining > 0 && !packing.deploy_target,
        "trial 10: deployed Looms took the order and never moved"
    );
    assert_eq!(packing.after_pack, Some(Order::Attack { target: enemy }));
    let mut deployed_in_reach = false;
    for _ in 0..1500 {
        world.step();
        let e = world.entity(loom).expect("loom");
        if e.deployed && e.pos.distance_sq(Pos::cell(far.0, far.1)) <= i64::from(range * FP).pow(2)
        {
            deployed_in_reach = true;
            break;
        }
    }
    assert!(deployed_in_reach, "it walks into reach and deploys to fire");
}

#[test]
fn a_deployed_loom_with_its_target_in_reach_stays_deployed() {
    let mut world = fixture(Faction::Assembly);
    let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
    world
        .issue(
            0,
            Command::SetDeployed {
                units: vec![loom],
                deployed: true,
            },
        )
        .expect("deploy");
    run(&mut world, 60);
    let near = reachable_east(&world, (40, 64), 4);
    let enemy = unit(&mut world, 1, Kind::Hook, near);
    world.step();
    assert!(world.entity_visible(0, enemy));
    world
        .issue(
            0,
            Command::Attack {
                units: vec![loom],
                target: enemy,
            },
        )
        .expect("attack");
    world.step();
    let e = world.entity(loom).expect("loom");
    assert!(e.deployed && e.deploy_remaining == 0);
    assert_eq!(e.order, Order::Attack { target: enemy });
}

#[test]
fn the_interface_can_ask_whether_any_machine_could_get_there() {
    let mut world = World::with_map(
        310,
        MapId::Confluence,
        &[Faction::Union, Faction::Assembly, Faction::Compact],
    )
    .expect("three seats");
    world.ai_enabled = false;
    let hq = world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
        .expect("own headquarters")
        .pos;
    let far = world
        .entities
        .iter()
        .find(|e| e.owner == 2 && e.kind == Kind::Headquarters)
        .expect("the third seat's headquarters")
        .pos;
    let (hx, hy) = hq.cell_xy();
    let mut riveter = None;
    'find: for dy in 5..12 {
        for dx in 5..12 {
            if world.cell_walkable_for(Kind::Riveter, (hx + dx, hy + dy)) {
                riveter = Some(world.spawn_unit(0, Kind::Riveter, Pos::cell(hx + dx, hy + dy)));
                break 'find;
            }
        }
    }
    let riveter = riveter.expect("room beside the headquarters");
    let home = Pos::cell(hx + 3, hy + 8);
    assert!(world.any_can_reach(&[riveter], home));
    // Every arm deep: nothing crosses to another bank.
    world.gate.tide = Tide::Flood;
    assert!(!world.any_can_reach(&[riveter], far));
    assert!(world.any_can_reach(&[riveter], home));
}
