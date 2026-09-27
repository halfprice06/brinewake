//! Rules 20: the trial 10 suggestions, tiers 1 and 2.  A flood freezes
//! every hold count; an enemy Defense Nest in a mouth ring blocks a count
//! but never holds one; each standing headquarters and the station's owner
//! earn salvage without a wreck; the Sounder hits a deployed Loom harder;
//! the Compact's Brander and Glinter carry more hull; a three-seat hold is
//! 75 s, and 120 s once a seat is out, when a lane to that seat no longer
//! counts.
use super::*;

fn basin() -> World {
    let mut world = World::new(2000, Faction::Union);
    world.ai_enabled = false;
    world
}

fn three_seats() -> World {
    let mut world = World::with_map(
        2001,
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

/// Hold both of the Split Basin's lanes for seat 0 from its west mouths.
fn hold_basin(world: &mut World) {
    world.gate.owner = Some(0);
    for mouths in CROSSING_MOUTHS {
        world.spawn_for_tests(0, Kind::Riveter, mouths[0]);
    }
}

fn knock_out(world: &mut World, seat: u8) {
    for entity in &mut world.entities {
        if entity.owner == seat && entity.kind == Kind::Headquarters {
            entity.hp = 0;
        }
    }
    world.entities.retain(|entity| entity.hp > 0);
    world.step();
    assert!(world.is_eliminated(seat), "seat {seat} went out");
}

fn no_workers(world: &mut World) {
    world.entities.retain(|entity| !entity.kind.is_worker());
}

#[test]
fn a_flood_freezes_every_count_and_each_resumes_when_it_falls() {
    let mut world = basin();
    hold_basin(&mut world);
    // Seat 1 has a count banked that drains while seat 0 holds.
    world.lane_hold[1] = 2000;
    run(&mut world, 100);
    // Rules 21: a broken count drains three ticks a tick.
    assert_eq!(
        world.lane_hold,
        vec![100, 2000 - 100 * TIDE_HOLD_DRAIN_PER_TICK]
    );
    world.players[0].pressure = world.players[0].pressure_cap;
    world.issue(0, Command::Flood).expect("the owner may flood");
    // The warning does not freeze: the counts still move.
    let mut guard = 0;
    while world.gate.flood_until.is_none() {
        world.step();
        guard += 1;
        assert!(guard <= FLOOD_WARNING_TICKS + 5, "the flood arrives");
    }
    assert!(world.hold_frozen());
    let frozen = world.lane_hold.clone();
    assert!(frozen[0] > 100 + FLOOD_WARNING_TICKS - 5, "{frozen:?}");
    assert!(world.holds_every_lane(0), "the guns still stand there");
    run(&mut world, FLOOD_TICKS / 2);
    assert_eq!(world.lane_hold, frozen, "neither rises nor drains");
    while world.gate.flood_until.is_some() {
        assert_eq!(world.lane_hold, frozen);
        world.step();
    }
    // The tick the flood falls the counts move again, from where they were.
    assert!(!world.hold_frozen());
    let drain = TIDE_HOLD_DRAIN_PER_TICK;
    assert_eq!(world.lane_hold, vec![frozen[0] + 1, frozen[1] - drain]);
    run(&mut world, 30);
    assert_eq!(world.lane_hold[0], frozen[0] + 31, "the count resumes");
    assert_eq!(
        world.lane_hold[1],
        frozen[1] - 31 * drain,
        "the drain resumes"
    );
}

#[test]
fn a_nest_in_a_mouth_ring_blocks_an_enemy_count_but_never_holds_one() {
    let mut world = basin();
    hold_basin(&mut world);
    assert!(world.holds_lane(0, 0) && world.holds_lane(0, 1));
    let far = CROSSING_MOUTHS[0][1];
    // A nest still being dug blocks nothing.
    let nest = world.spawn_for_tests(1, Kind::Tower, far);
    world.entity_mut(nest).expect("nest").build_remaining = 10;
    assert!(world.holds_lane(0, 0));
    // A finished one blocks the lane as an enemy gun would.
    world.entity_mut(nest).expect("nest").build_remaining = 0;
    assert!(world.mouth_blocked_for(0, far));
    assert!(!world.holds_lane(0, 0));
    assert!(!world.holds_crossing(0, far));
    assert!(world.holds_lane(0, 1), "only its own lane");
    run(&mut world, 60);
    assert_eq!(world.lane_hold[0], 0, "no count runs past a nest");
    // Outside the ring it blocks nothing.
    world.entity_mut(nest).expect("nest").pos = Pos::cell(
        far.cell_xy().0 + CROSSING_HOLD_RADIUS_CELLS + 2,
        far.cell_xy().1,
    );
    assert!(world.holds_lane(0, 0));

    // A nest never makes its own side's hold, even with the sluice.
    let mut world = basin();
    world.gate.owner = Some(0);
    world.spawn_for_tests(0, Kind::Tower, CROSSING_MOUTHS[0][0]);
    world.spawn_for_tests(0, Kind::Riveter, CROSSING_MOUTHS[1][0]);
    assert!(!world.holds_lane(0, 0), "an own nest holds nothing");
    assert!(!world.mouth_blocked_for(0, CROSSING_MOUTHS[0][0]));
    assert!(!world.holds_every_lane(0));
    // And it blocks the other side's count at that mouth.
    world.gate.owner = Some(1);
    world.spawn_for_tests(1, Kind::Reedguard, CROSSING_MOUTHS[0][1]);
    assert!(!world.holds_lane(1, 0));
}

#[test]
fn each_standing_headquarters_yields_forty_salvage_a_minute() {
    let mut world = basin();
    no_workers(&mut world);
    let before: Vec<u32> = world.players.iter().map(|p| p.salvage).collect();
    run(&mut world, 60 * 30);
    for (player, start) in before.iter().enumerate() {
        assert_eq!(
            world.players[player].salvage - start,
            HQ_SALVAGE_PER_MINUTE,
            "seat {player}"
        );
    }
    // A seat that is out earns nothing more, whatever it had.
    let mut world = three_seats();
    no_workers(&mut world);
    knock_out(&mut world, 2);
    let out = world.players[2].salvage;
    let standing = world.players[1].salvage;
    run(&mut world, 60 * 30);
    assert_eq!(world.players[2].salvage, out);
    assert_eq!(world.players[1].salvage - standing, HQ_SALVAGE_PER_MINUTE);
}

#[test]
fn the_sluice_owner_gains_thirty_salvage_a_minute() {
    let mut world = basin();
    no_workers(&mut world);
    world.gate.owner = Some(1);
    let before: Vec<u32> = world.players.iter().map(|p| p.salvage).collect();
    run(&mut world, 60 * 30);
    assert_eq!(world.players[0].salvage - before[0], HQ_SALVAGE_PER_MINUTE);
    assert_eq!(
        world.players[1].salvage - before[1],
        HQ_SALVAGE_PER_MINUTE + STATION_SALVAGE_PER_MINUTE
    );
    // The trickle stops when the station changes hands.  (Pressure is
    // emptied so none spills over the cap into salvage.)
    world.gate.owner = None;
    world.players[1].pressure = 0;
    let now = world.players[1].salvage;
    run(&mut world, 60 * 30);
    assert_eq!(world.players[1].salvage - now, HQ_SALVAGE_PER_MINUTE);
}

#[test]
fn a_sounder_hits_a_deployed_loom_for_five_more() {
    let mut world = basin();
    let sounder = world.spawn_for_tests(0, Kind::Sounder, Pos::cell(40, 60));
    let loom = world.spawn_for_tests(1, Kind::Loom, Pos::cell(42, 60));
    let shot = spec(Kind::Sounder).damage;
    assert_eq!(world.modified_damage(sounder, loom, shot), shot);
    world.entity_mut(loom).expect("loom").deployed = true;
    assert_eq!(SOUNDER_LOOM_BONUS, 5);
    assert_eq!(world.modified_damage(sounder, loom, shot), 12);
}

#[test]
fn the_compacts_brander_and_glinter_carry_more_hull() {
    // Brander 110 + 15%, rounded; the Glinter 80 to 90 and its gun as it
    // was (8 damage, reach 5).
    assert_eq!(spec(Kind::Brander).health, 127);
    assert_eq!(spec(Kind::Brander).salvage, 75);
    assert_eq!(spec(Kind::Glinter).health, 90);
    assert_eq!(spec(Kind::Glinter).damage, 8);
    assert_eq!(spec(Kind::Glinter).range, 5 * FP);
}

#[test]
fn a_three_seat_hold_is_75_seconds_and_120_once_a_seat_is_out() {
    assert_eq!(basin().hold_ticks(), 90 * 30);
    let mut world = three_seats();
    assert_eq!(world.hold_ticks(), 75 * 30);
    world.gate.owner = Some(0);
    for arm in world.arms_of(0) {
        let mouth = world.own_mouth(0, arm).expect("own bank");
        world.spawn_for_tests(0, Kind::Riveter, mouth);
    }
    run(&mut world, 75 * 30 - 1);
    assert_eq!(world.outcome, None, "one tick short of 75 s");
    world.step();
    assert_eq!(world.outcome, Some(Outcome::Victory(0)));

    let mut world = three_seats();
    knock_out(&mut world, 2);
    assert_eq!(world.hold_ticks(), 120 * 30);
    assert_eq!(hold_ticks_for(3, true), TIDE_HOLD_AFTER_OUT_TICKS);
    assert_eq!(hold_ticks_for(2, false), TIDE_HOLD_TICKS);
}

#[test]
fn once_a_seat_is_out_a_hold_needs_only_the_lanes_to_seats_still_standing() {
    let mut world = three_seats();
    let arms = world.arms_of(0);
    assert_eq!(world.hold_arms(0), arms, "all three stand: both lanes");
    // Seat 2 goes out: the lane between seats 0 and 2 no longer counts.
    let dead = world.arm_between(0, 2).expect("a lane to seat 2");
    let live = world.arm_between(0, 1).expect("a lane to seat 1");
    knock_out(&mut world, 2);
    assert_eq!(world.hold_arms(0), vec![live]);
    assert_eq!(world.hold_arms(1), vec![live]);
    world.gate.owner = Some(0);
    world.spawn_for_tests(0, Kind::Riveter, world.own_mouth(0, live).expect("bank"));
    // Seat 1's gun on the dead lane's mouth contests nothing any more.
    let [a, b] = world.map.layout().arm_mouths(dead);
    world.spawn_for_tests(1, Kind::Riveter, a);
    world.spawn_for_tests(1, Kind::Riveter, b);
    assert!(!world.holds_lane(0, dead));
    assert!(world.holds_every_lane(0));
    run(&mut world, 90 * 30);
    assert_eq!(world.outcome, None, "the hold is 120 s now");
    run(&mut world, 30 * 30);
    assert_eq!(world.outcome, Some(Outcome::Victory(0)));
    assert_eq!(world.lane_hold[0], TIDE_HOLD_AFTER_OUT_TICKS);
}

#[test]
fn the_split_basin_hold_still_needs_both_lanes() {
    let mut world = basin();
    assert_eq!(world.hold_arms(0), vec![0, 1]);
    assert_eq!(world.hold_arms(1), vec![0, 1]);
    world.gate.owner = Some(0);
    world.spawn_for_tests(0, Kind::Riveter, CROSSING_MOUTHS[0][0]);
    assert!(!world.holds_every_lane(0));
}

#[test]
fn the_practice_ai_holds_only_lanes_to_standing_seats_and_sees_a_gun_as_a_denial() {
    let mut world = three_seats();
    assert_eq!(world.ai_arms(1), world.hold_arms(1));
    assert_eq!(world.ai_denied_arm(1), None);
    // A finished nest of seat 0's at one of the AI's far mouths denied it
    // under rules 20; with three seats it only slows the count since rules
    // 22, so the lane can still be held. An enemy gun there still denies.
    let arm = world.arm_between(1, 0).expect("a lane to seat 0");
    let far = world.ai_far_mouth(1, arm);
    world.spawn_for_tests(0, Kind::Tower, far);
    assert_eq!(world.ai_denied_arm(1), None);
    assert!(world.mouth_slowed_for(1, far));
    world.spawn_for_tests(0, Kind::Riveter, far);
    assert_eq!(world.ai_denied_arm(1), Some(arm));
    // Once seat 2 is out, the AI no longer garrisons the lane to it.
    let mut world = three_seats();
    let live = world.arm_between(1, 0).expect("lane");
    knock_out(&mut world, 2);
    assert_eq!(world.ai_arms(1), vec![live]);
    assert_eq!(world.ai_mouths(1), vec![world.ai_near_mouth(1, live)]);
}

#[test]
fn the_practice_ai_floods_an_attack_caught_in_the_lanes() {
    let mut world = basin();
    world.ai_enabled = true;
    hold_basin_for(&mut world, 1);
    world.players[1].pressure = world.players[1].pressure_cap;
    // An empty lane is no reason to freeze its own count.
    run(&mut world, 300);
    assert!(
        !world
            .command_log
            .iter()
            .any(|record| record.player == 1 && matches!(record.command, Command::Flood)),
        "no flood against nothing"
    );
    assert!(world.lane_hold[1] > 0, "its count runs");
    // Three of seat 0's guns wade the north lane toward its mouth.
    let lane = (60..70)
        .flat_map(|x| (40..60).map(move |y| (x, y)))
        .filter(|&(x, y)| world.map.terrain(x, y).is_tidal() && world.visible(1, Pos::cell(x, y)))
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(lane.len(), 3, "three wet cells the AI can see");
    for (x, y) in lane {
        let id = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(x, y));
        world.entity_mut(id).expect("riveter").order = Order::Hold;
    }
    run(&mut world, 120);
    assert!(
        world
            .command_log
            .iter()
            .any(|record| record.player == 1 && matches!(record.command, Command::Flood)),
        "the AI swamps the attack"
    );
}

/// Seat `player` of the Split Basin holds both lanes from its own mouths.
fn hold_basin_for(world: &mut World, player: u8) {
    world.gate.owner = Some(player);
    for arm in 0..2 {
        let mouth = world.ai_near_mouth(player, arm);
        for _ in 0..2 {
            world.spawn_for_tests(player, Kind::Riveter, mouth);
        }
    }
}
