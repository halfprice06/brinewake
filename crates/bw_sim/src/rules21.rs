//! Rules 21: the trial 11 design changes.  The Glinter hits a deployed
//! Loom as hard as the Sounder does and the Loom reaches seven cells, eight
//! with SIEGE; a broken hold count drains three ticks a tick; OVERHAUL, a
//! three-level salvage sink at the headquarters, adds 6% hull a level to
//! every machine the side owns.
use super::*;

fn basin(faction: Faction) -> World {
    let mut world = World::with_map(2100, MapId::SplitBasin, &[faction, Faction::Assembly])
        .expect("two seats fit the Split Basin");
    world.ai_enabled = false;
    world
}

fn confluence() -> World {
    let mut world = World::with_map(
        2101,
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
fn rules_21_is_a_new_rules_version_with_its_own_digest() {
    // Rules 22 came after; its own test pins the number.
    const { assert!(bw_content::RULES_VERSION >= 21) };
    assert_eq!(GLINTER_LOOM_BONUS, SOUNDER_LOOM_BONUS);
    assert_eq!(TIDE_HOLD_DRAIN_PER_TICK, 3);
    assert_eq!(OVERHAUL_HULL_PERCENT, 6);
    assert_eq!(bw_content::rules_digest(), bw_content::rules_digest());
}

#[test]
fn a_glinter_hits_a_deployed_loom_for_five_more_on_both_maps() {
    for mut world in [basin(Faction::Compact), confluence()] {
        let (glinter_seat, loom_seat) = if world.seat_count() > 2 {
            (2, 1)
        } else {
            (0, 1)
        };
        let glinter = world.spawn_for_tests(glinter_seat, Kind::Glinter, Pos::cell(40, 60));
        let brander = world.spawn_for_tests(glinter_seat, Kind::Brander, Pos::cell(40, 61));
        let loom = world.spawn_for_tests(loom_seat, Kind::Loom, Pos::cell(42, 60));
        let wet = world
            .entity(loom)
            .is_some_and(|e| world.wading(e) || world.swamped(e));
        assert!(!wet, "the test Loom stands on dry ground");
        let shot = spec(Kind::Glinter).damage;
        assert_eq!(world.modified_damage(glinter, loom, shot), shot, "packed");
        world.entity_mut(loom).expect("loom").deployed = true;
        assert_eq!(
            world.modified_damage(glinter, loom, shot),
            shot + GLINTER_LOOM_BONUS
        );
        assert_eq!(world.modified_damage(glinter, loom, shot), 13);
        // Only the Loom hunters: a Brander's shot is unchanged.
        let cut = spec(Kind::Brander).damage;
        assert_eq!(world.modified_damage(brander, loom, cut), cut);
        // And the bonus is for a Loom, not another deployed machine.
        let other = world.spawn_for_tests(loom_seat, Kind::Heliostat, Pos::cell(42, 62));
        world.entity_mut(other).expect("gun").deployed = true;
        let wet = world
            .entity(other)
            .is_some_and(|e| world.wading(e) || world.swamped(e));
        if !wet {
            assert_eq!(world.modified_damage(glinter, other, shot), shot);
        }
    }
}

#[test]
fn a_loom_reaches_seven_cells_and_eight_with_siege() {
    assert_eq!(spec(Kind::Loom).range, 7 * FP);
    let mut world = basin(Faction::Assembly);
    let loom = world.spawn_for_tests(0, Kind::Loom, Pos::cell(40, 60));
    let reach = |world: &World| world.weapon_range(world.entity(loom).expect("loom"));
    assert_eq!(reach(&world), 7 * FP);
    world.players[0].upgrades.push(Upgrade::Siege);
    assert_eq!(reach(&world), 8 * FP);
    // A Glinter (5) and a Heliostat (6) are still outranged, but a
    // sieged Loom no longer reaches past a line machine's sight (8).
    assert!(spec(Kind::Heliostat).range < reach(&world));
    assert_eq!(spec(Kind::Brander).sight, reach(&world));
}

#[test]
fn a_broken_count_drains_three_seconds_a_second_on_every_map() {
    // The Split Basin, two seats.
    let mut world = basin(Faction::Union);
    world.lane_hold[0] = 60 * 30;
    run(&mut world, 30);
    assert_eq!(
        world.lane_hold[0],
        60 * 30 - 3 * 30,
        "one second takes three"
    );
    run(&mut world, 57 * 30);
    assert_eq!(world.lane_hold[0], 0, "it empties and stops at zero");
    run(&mut world, 30);
    assert_eq!(world.lane_hold[0], 0);

    // The Confluence, three seats and then two.
    let mut world = confluence();
    world.lane_hold = vec![50 * 30, 40 * 30, 30 * 30];
    run(&mut world, 10 * 30);
    assert_eq!(world.lane_hold, vec![20 * 30, 10 * 30, 0]);
    // A held count still rises one tick a tick.
    hold_everything(&mut world, 0);
    run(&mut world, 30);
    assert_eq!(world.lane_hold[0], 20 * 30 + 30);
    assert_eq!(world.lane_hold[1], 10 * 30 - 90);
}

#[test]
fn a_flood_still_freezes_a_draining_count() {
    for mut world in [basin(Faction::Union), confluence()] {
        let seats = world.seat_count();
        world.lane_hold = vec![40 * 30; seats];
        world.gate.tide = Tide::Flood;
        world.gate.flood_until = Some(world.tick + 20 * 30);
        assert!(world.hold_frozen());
        run(&mut world, 10 * 30);
        assert_eq!(
            world.lane_hold,
            vec![40 * 30; seats],
            "no drain while frozen"
        );
    }
}

#[test]
fn overhaul_costs_salvage_only_and_its_levels_come_in_order() {
    assert_eq!(Upgrade::Overhaul1.cost(), (1_000, 0, 45 * 30));
    assert_eq!(Upgrade::Overhaul2.cost(), (1_500, 0, 45 * 30));
    assert_eq!(Upgrade::Overhaul3.cost(), (2_000, 0, 45 * 30));
    for level in Upgrade::OVERHAUL {
        assert_eq!(level.building(), Kind::Headquarters);
    }
    assert_eq!(Upgrade::offered_by(Kind::Headquarters), &Upgrade::OVERHAUL);

    let mut world = basin(Faction::Union);
    let base = hq(&world, 0);
    world.players[0].salvage = 10_000;
    world.players[0].pressure = 0;
    // Level II before level I is refused.
    let refused = world.issue(
        0,
        Command::Upgrade {
            building: base,
            upgrade: Upgrade::Overhaul2,
        },
    );
    assert!(
        refused.is_err_and(|why| why.contains("needs OVERHAUL I")),
        "level II needs level I"
    );
    // All three queue in order at the headquarters, with no pressure.
    for level in Upgrade::OVERHAUL {
        world
            .issue(
                0,
                Command::Upgrade {
                    building: base,
                    upgrade: level,
                },
            )
            .expect("the next level queues");
        world.step();
    }
    assert_eq!(world.players[0].salvage_spent, 4_500);
    assert_eq!(world.players[0].pressure_spent, 0);
    let entity = world.entity(base).expect("hq");
    assert_eq!(
        entity.upgrade.map(|job| job.upgrade),
        Some(Upgrade::Overhaul1)
    );
    assert_eq!(
        entity.upgrade_queue,
        vec![Upgrade::Overhaul2, Upgrade::Overhaul3]
    );
    // A cancel takes the last level first and gives all of it back.
    let before = world.players[0].salvage;
    world
        .issue(0, Command::CancelUpgrade { building: base })
        .expect("cancel");
    world.step();
    // (The headquarters' own trickle may land in the same tick.)
    let back = world.players[0].salvage - before;
    assert!((2_000..=2_001).contains(&back), "{back}");
    assert_eq!(
        world.entity(base).expect("hq").upgrade_queue,
        vec![Upgrade::Overhaul2]
    );
    // Level I completes after 45 s, then level II starts.
    run(&mut world, 45 * 30);
    assert_eq!(world.overhaul_level(0), 1);
    assert_eq!(
        world.entity(base).expect("hq").upgrade.map(|j| j.upgrade),
        Some(Upgrade::Overhaul2)
    );
    run(&mut world, 45 * 30);
    assert_eq!(world.overhaul_level(0), 2);
}

#[test]
fn overhaul_is_not_part_of_a_doctrine_lock() {
    let mut world = basin(Faction::Union);
    let base = hq(&world, 0);
    world.players[0].doctrine = Some(Doctrine::FireControl);
    world.players[0].doctrine_tier = 2;
    world.players[0].salvage = 1_000;
    world
        .issue(
            0,
            Command::Upgrade {
                building: base,
                upgrade: Upgrade::Overhaul1,
            },
        )
        .expect("a doctrine does not lock OVERHAUL");
}

#[test]
fn each_overhaul_level_adds_six_percent_hull_to_every_machine_built_or_not() {
    for mut world in [basin(Faction::Union), confluence()] {
        let seats = world.seat_count() as u8;
        let seat = seats - 1;
        let faction = world.players[seat as usize].faction;
        let worker = faction.worker();
        let fighter = faction.army()[0];
        let old = world.spawn_for_tests(seat, fighter, Pos::cell(40, 60));
        let hurt = world.spawn_for_tests(seat, worker, Pos::cell(41, 60));
        world.entity_mut(hurt).expect("worker").hp = 20;
        let base = hq(&world, seat);
        let hq_hull = world.entity(base).expect("hq").max_hp;
        world.players[seat as usize]
            .upgrades
            .push(Upgrade::Overhaul1);
        world.apply_upgrade_effects(seat, Upgrade::Overhaul1);
        let six = |kind: Kind| (spec(kind).health * 6 + 50) / 100;
        let fighter_hull = spec(fighter).health + six(fighter);
        let entity = world.entity(old).expect("fighter");
        assert_eq!(entity.max_hp, fighter_hull, "{fighter:?}");
        assert_eq!(entity.hp, fighter_hull, "raised by the same amount");
        let entity = world.entity(hurt).expect("worker");
        assert_eq!(entity.max_hp, spec(worker).health + six(worker));
        assert_eq!(entity.hp, 20 + six(worker), "a hurt machine gains the same");
        // Buildings are unaffected.
        assert_eq!(world.entity(base).expect("hq").max_hp, hq_hull);
        // A machine built after it has it too, and the levels add up.
        world.players[seat as usize]
            .upgrades
            .push(Upgrade::Overhaul2);
        world.players[seat as usize]
            .upgrades
            .push(Upgrade::Overhaul3);
        let three = (spec(fighter).health * 18 + 50) / 100;
        assert_eq!(
            world.unit_max_hp(seat, fighter),
            spec(fighter).health + three
        );
        // Another seat's machines are untouched.
        assert_eq!(world.unit_max_hp(0, Kind::Hook), spec(Kind::Hook).health);
    }
    // Stacks with REFIT and PLATE: a Riveter 125 + 25% + 15% + 6% (8).
    let mut world = basin(Faction::Union);
    world.players[0].upgrades = vec![Upgrade::Plate, Upgrade::Refit, Upgrade::Overhaul1];
    assert_eq!(world.unit_max_hp(0, Kind::Riveter), 175 + 8);
}

#[test]
fn a_completed_overhaul_raises_the_hull_of_machines_in_the_field() {
    let mut world = basin(Faction::Union);
    let base = hq(&world, 0);
    let riveter = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(40, 60));
    world.players[0].salvage = 1_000;
    world
        .issue(
            0,
            Command::Upgrade {
                building: base,
                upgrade: Upgrade::Overhaul1,
            },
        )
        .expect("level I");
    run(&mut world, 45 * 30 + 1);
    assert!(world.players[0].upgrades.contains(&Upgrade::Overhaul1));
    let entity = world.entity(riveter).expect("riveter");
    assert_eq!(entity.max_hp, 125 + 8);
}

#[test]
fn the_practice_ai_buys_overhaul_only_with_twice_its_price_banked() {
    let mut world = basin(Faction::Union);
    let base = hq(&world, 1);
    let works = world.spawn_for_tests(1, Kind::Works, Pos::cell(100, 60));
    world
        .entity_mut(works)
        .expect("works")
        .queue
        .push(Production {
            kind: Kind::Loom,
            remaining: 1_000,
            started: false,
            cost_salvage: 0,
            cost_pressure: 0,
        });
    let own: Vec<Entity> = world
        .entities
        .iter()
        .filter(|e| e.owner == 1)
        .cloned()
        .collect();
    let hq_entity = world.entity(base).expect("hq").clone();
    world.players[1].salvage = 2_000;
    world.ai_overhaul(1, &own, &hq_entity);
    assert!(
        world.entity(base).expect("hq").upgrade.is_none(),
        "2,000 is not more than twice 1,000"
    );
    world.players[1].salvage = 2_001;
    world.ai_overhaul(1, &own, &hq_entity);
    world.step();
    assert_eq!(
        world.entity(base).expect("hq").upgrade.map(|j| j.upgrade),
        Some(Upgrade::Overhaul1)
    );
    // With its Works idle and crew to spare it keeps the salvage for them.
    let mut world = basin(Faction::Union);
    let base = hq(&world, 1);
    world.spawn_for_tests(1, Kind::Works, Pos::cell(100, 60));
    let own: Vec<Entity> = world
        .entities
        .iter()
        .filter(|e| e.owner == 1)
        .cloned()
        .collect();
    let hq_entity = world.entity(base).expect("hq").clone();
    world.players[1].salvage = 9_000;
    assert!(world.players[1].crew < world.players[1].cap);
    world.ai_overhaul(1, &own, &hq_entity);
    world.step();
    assert!(world.entity(base).expect("hq").upgrade.is_none());
    // At the crew ceiling there is nothing else to buy.
    world.players[1].crew = world.players[1].cap;
    world.ai_overhaul(1, &own, &hq_entity);
    world.step();
    assert!(world.entity(base).expect("hq").upgrade.is_some());
}
