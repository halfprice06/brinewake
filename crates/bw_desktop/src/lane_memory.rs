//! Lane memory: a drained lane dries to salt; a flood arrives as a front.
//!
//! Everything here derives from two public facts: the current gate state and
//! the tick of the last authoritative `GateChanged` event.  The drying stage
//! chooses a tile family for the lane that just drained; the front distance
//! places a foam crest travelling out from the arm's centre line along the
//! lane that just flooded, and a receding sheen on the lane that drained.
//! Path cost and the tile swap follow the authoritative state on the change
//! tick; only these decorations animate around it.
//!
//! Every map's arms are read from its layout: an arm runs from the station
//! out toward its lane's centre, and its lane is a band across it.  On the
//! Split Basin the arms run north and south of the sluice spine; on the
//! Confluence three arms run diagonally out from the station island.

use bw_core::TICK_HZ;
use bw_sim::{Arm, Depth, Gate, Layout, MapId, Tide, World};

/// Each drying stage holds for fifteen seconds; four stages, then dry.
pub const STAGE_TICKS: u64 = 15 * TICK_HZ;
pub const STAGES: u64 = 4;
/// The flood front and the draining sheen cross the lane in two seconds.
pub const FRONT_TICKS: u64 = 2 * TICK_HZ;
/// The Split Basin's lanes run from x 52 to 75 either side of the spine at
/// x 62..=66, so the farthest lane cell is nine cells from the spine edge.
#[cfg(test)]
const SPINE_LEFT: i32 = 62;
#[cfg(test)]
const SPINE_RIGHT: i32 = 66;
pub const LANE_REACH: i32 = 9;
/// The most arms a map has: the tide state is kept arm by arm up to this.
pub const MAX_ARMS: usize = 3;

/// Drying stage for the lane that just drained, or `None` once it is dry.
pub fn drying_stage(changed: Option<u64>, tick: u64) -> Option<u8> {
    let age = tick.checked_sub(changed?)?;
    let stage = age / STAGE_TICKS;
    (stage < STAGES).then_some(stage as u8)
}

/// Atlas key for a drained lane cell during its drying minute.
pub fn drying_key(stage: u8, variant: u64) -> String {
    format!("terrain_lane_drying_{}_{}", stage.min(3), variant % 4)
}

/// Cells from the Split Basin's spine edge along a lane: 0 beside the spine.
/// `crossing_distance` gives the same on the Split Basin; the tests hold it.
#[cfg(test)]
pub fn spine_distance(x: i32) -> i32 {
    if x < SPINE_LEFT {
        SPINE_LEFT - 1 - x
    } else if x > SPINE_RIGHT {
        x - SPINE_RIGHT - 1
    } else {
        -1
    }
}

/// Half the width of the still water along an arm's centre line, where no
/// front runs: the Split Basin's sluice spine is five cells wide, and the
/// Confluence's fronts start on the centre line itself.
fn spine_half_width(map: MapId) -> f64 {
    match map {
        MapId::SplitBasin => 2.5,
        MapId::Confluence => 0.0,
    }
}

/// Cells from an arm's centre line to a cell, measured along the arm's
/// crossing: 0 beside the spine, -1 on it, `None` for an arm the map lacks.
/// The centre line runs from the station toward the arm's lane centre, so
/// on the Split Basin this is `spine_distance(x)` exactly.
pub fn crossing_distance(layout: &Layout, arm: usize, x: i32, y: i32) -> Option<i32> {
    let centre = layout.arms.get(arm)?.centre;
    let (sx, sy) = layout.station;
    let (ax, ay) = (f64::from(centre.0 - sx), f64::from(centre.1 - sy));
    let length = ax.hypot(ay);
    if length == 0.0 {
        return None;
    }
    // The unit vector across the arm, perpendicular to its centre line.
    let across = (-ay / length, ax / length);
    let offset = (f64::from(x - sx) * across.0 + f64::from(y - sy) * across.1).abs();
    Some(((offset - spine_half_width(layout.id)).floor() as i32).max(-1))
}

/// The farthest tidal cell of an arm from its centre line: how far the
/// fronts travel.  `LANE_REACH` on the Split Basin.
pub fn arm_reach(world: &World, arm: usize) -> i32 {
    let layout = world.map.layout();
    let mut reach = 0;
    for y in 0..i32::from(world.map.height) {
        for x in 0..i32::from(world.map.width) {
            if world.map.terrain(x, y).tidal_arm().map(usize::from) == Some(arm)
                && let Some(d) = crossing_distance(layout, arm, x, y)
            {
                reach = reach.max(d);
            }
        }
    }
    reach
}

/// Front position in cells from the spine for the flooding lane, or `None`
/// once the front has passed the lane end.
pub fn flood_front(changed: Option<u64>, tick: u64) -> Option<i32> {
    flood_front_within(changed, tick, LANE_REACH)
}

/// The flood front on an arm whose farthest cell is `reach` from the spine.
pub fn flood_front_within(changed: Option<u64>, tick: u64, reach: i32) -> Option<i32> {
    let age = tick.checked_sub(changed?)?;
    let steps = reach.max(0) as u64 + 1;
    (age < FRONT_TICKS).then(|| (age * steps / FRONT_TICKS) as i32)
}

/// Sheen position for the draining lane: starts at the lane end and recedes
/// toward the spine over the same window.
#[cfg(test)]
pub fn drain_front(changed: Option<u64>, tick: u64) -> Option<i32> {
    drain_front_within(changed, tick, LANE_REACH)
}

/// The draining sheen on an arm whose farthest cell is `reach` from the spine.
pub fn drain_front_within(changed: Option<u64>, tick: u64, reach: i32) -> Option<i32> {
    flood_front_within(changed, tick, reach).map(|d| reach.max(0) - d)
}

/// The water on every arm at a tide, arm by arm.
pub fn arm_depths(tide: Tide, dry_arm: Arm) -> [Depth; MAX_ARMS] {
    std::array::from_fn(|arm| match tide {
        Tide::Neutral => Depth::Shallow,
        Tide::Flood => Depth::Deep,
        Tide::Open => {
            if arm == dry_arm.index() {
                Depth::Dry
            } else {
                Depth::Deep
            }
        }
    })
}

/// The water on each side, north then south, at a tide: the Split Basin's
/// two arms.
pub fn depths(tide: Tide, north_dry: bool) -> [Depth; 2] {
    let all = arm_depths(tide, Arm::from_north(north_dry));
    [all[0], all[1]]
}

/// The last tide change: when, and the water on each arm before and after,
/// so the drying stages and the foam front follow the arm that actually
/// changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TideChange {
    pub tick: u64,
    pub from: [Depth; MAX_ARMS],
    pub to: [Depth; MAX_ARMS],
}

impl TideChange {
    /// From the gate after a `GateChanged` event and the event's text, when
    /// the tide before it is not known.
    pub fn from_change(gate: &Gate, text: &str, tick: u64) -> Self {
        let to = arm_depths(gate.tide, gate.dry_arm);
        let from = match text {
            "flood" => {
                if gate.opened {
                    arm_depths(Tide::Open, gate.dry_arm)
                } else {
                    arm_depths(Tide::Neutral, gate.dry_arm)
                }
            }
            "tide falls" => arm_depths(Tide::Flood, gate.dry_arm),
            // An open: the open arm dries and the others flood whatever
            // they were before, so all read as changed from shallow.
            _ => [Depth::Shallow; MAX_ARMS],
        };
        Self { tick, from, to }
    }

    /// From the tide just before a change and the gate just after it: an
    /// arm changes only if its water did, so an arm that was deep and stays
    /// deep through an open shows no front, and an open of the arm that was
    /// already dry dries nothing again.
    pub fn observed(before_tide: Tide, before_dry: Arm, gate: &Gate, tick: u64) -> Self {
        Self {
            tick,
            from: arm_depths(before_tide, before_dry),
            to: arm_depths(gate.tide, gate.dry_arm),
        }
    }

    /// The arm is dry now and was not.
    pub fn dried(&self, arm: usize) -> bool {
        arm < MAX_ARMS && self.to[arm] == Depth::Dry && self.from[arm] != Depth::Dry
    }
    /// The arm is deep now and was not.
    pub fn flooded(&self, arm: usize) -> bool {
        arm < MAX_ARMS && self.to[arm] == Depth::Deep && self.from[arm] != Depth::Deep
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{Faction, Terrain};

    #[test]
    fn drying_runs_four_stages_then_stops() {
        assert_eq!(drying_stage(None, 500), None);
        assert_eq!(drying_stage(Some(100), 99), None);
        assert_eq!(drying_stage(Some(100), 100), Some(0));
        assert_eq!(drying_stage(Some(100), 100 + STAGE_TICKS - 1), Some(0));
        assert_eq!(drying_stage(Some(100), 100 + STAGE_TICKS), Some(1));
        assert_eq!(drying_stage(Some(100), 100 + 3 * STAGE_TICKS), Some(3));
        assert_eq!(drying_stage(Some(100), 100 + 4 * STAGE_TICKS), None);
        assert_eq!(drying_key(2, 7), "terrain_lane_drying_2_3");
    }

    #[test]
    fn fronts_cross_the_lane_in_two_seconds_and_never_run_past_it() {
        assert_eq!(flood_front(Some(10), 10), Some(0));
        let mut last = -1;
        for age in 0..FRONT_TICKS {
            let d = flood_front(Some(10), 10 + age).unwrap();
            assert!(d >= last && d <= LANE_REACH, "{d}");
            last = d;
        }
        assert_eq!(flood_front(Some(10), 10 + FRONT_TICKS), None);
        assert_eq!(drain_front(Some(10), 10), Some(LANE_REACH));
        assert_eq!(spine_distance(61), 0);
        assert_eq!(spine_distance(67), 0);
        assert_eq!(spine_distance(52), LANE_REACH);
        assert_eq!(spine_distance(75), LANE_REACH - 1);
        assert_eq!(spine_distance(64), -1);
        let mut gate = bw_sim::World::new(1, bw_core::Faction::Union).gate;
        gate.tide = Tide::Open;
        gate.dry_arm = bw_sim::Arm::from_north(true);
        gate.opened = true;
        let change = TideChange::from_change(&gate, "north open", 10);
        assert!(change.dried(0) && change.flooded(1));
        gate.tide = Tide::Flood;
        let flood = TideChange::from_change(&gate, "flood", 20);
        assert!(
            flood.flooded(0) && !flood.flooded(1),
            "the deep side was deep already"
        );
        gate.tide = Tide::Open;
        let ebb = TideChange::from_change(&gate, "tide falls", 30);
        assert!(ebb.dried(0) && !ebb.flooded(1));
    }

    #[test]
    fn the_split_basin_crossing_is_the_spine_distance() {
        let world = World::new(1, Faction::Union);
        let layout = world.map.layout();
        for arm in 0..2 {
            for y in 0..128 {
                for x in 0..128 {
                    assert_eq!(
                        crossing_distance(layout, arm, x, y),
                        Some(spine_distance(x)),
                        "arm {arm} ({x},{y})"
                    );
                }
            }
            assert_eq!(arm_reach(&world, arm), LANE_REACH);
        }
        assert_eq!(crossing_distance(layout, 2, 60, 49), None);
        for age in 0..=FRONT_TICKS {
            assert_eq!(
                flood_front_within(Some(0), age, LANE_REACH),
                flood_front(Some(0), age)
            );
            assert_eq!(
                drain_front_within(Some(0), age, LANE_REACH),
                drain_front(Some(0), age)
            );
        }
        assert_eq!(depths(Tide::Open, true), [Depth::Dry, Depth::Deep]);
        assert_eq!(depths(Tide::Open, false), [Depth::Deep, Depth::Dry]);
        // On two arms an observed change reads as the event text did.
        let mut gate = world.gate.clone();
        gate.tide = Tide::Open;
        gate.dry_arm = Arm::SOUTH;
        let seen = TideChange::observed(Tide::Open, Arm::NORTH, &gate, 5);
        let told = TideChange::from_change(&gate, "south open", 5);
        for arm in 0..2 {
            assert_eq!(seen.dried(arm), told.dried(arm));
            assert_eq!(seen.flooded(arm), told.flooded(arm));
        }
    }

    fn confluence() -> World {
        World::with_map(
            1,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .expect("the Confluence starts")
    }

    #[test]
    fn confluence_fronts_run_across_each_diagonal_arm() {
        let world = confluence();
        let layout = world.map.layout();
        for (arm, arm_layout) in layout.arms.iter().enumerate() {
            // The lane's centre is on the centre line; its two mouths lie on
            // either bank, just past the farthest lane cell.
            let (cx, cy) = arm_layout.centre;
            assert!(crossing_distance(layout, arm, cx, cy).unwrap() <= 1);
            let reach = arm_reach(&world, arm);
            assert!((6..=16).contains(&reach), "arm {arm} reach {reach}");
            for (mx, my) in arm_layout.mouths {
                let d = crossing_distance(layout, arm, mx, my).unwrap();
                assert!(
                    (reach - 4..=reach + 4).contains(&d),
                    "arm {arm} mouth {d} of {reach}"
                );
            }
            let mut lane = 0;
            for y in 0..176 {
                for x in 0..176 {
                    if world.map.terrain(x, y) == Terrain::lane(arm as u8) {
                        lane += 1;
                        let d = crossing_distance(layout, arm, x, y).unwrap();
                        assert!((0..=reach).contains(&d), "({x},{y}) {d}");
                    }
                }
            }
            assert!(lane > 60, "arm {arm} has {lane} lane cells");
        }
        // E and W are mirror images across the diagonal.
        assert_eq!(arm_reach(&world, 0), arm_reach(&world, 1));
    }

    #[test]
    fn an_observed_open_changes_only_the_arms_whose_water_changed() {
        let mut world = confluence();
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm(1);
        // E was dry: E floods, W dries, S was deep and stays deep.
        let change = TideChange::observed(Tide::Open, Arm(0), &world.gate, 50);
        assert!(change.flooded(0) && change.dried(1));
        assert!(!change.flooded(2) && !change.dried(2));
        // From the neutral tide every arm changes.
        let first = TideChange::observed(Tide::Neutral, Arm(0), &world.gate, 50);
        assert!(first.flooded(0) && first.dried(1) && first.flooded(2));
        // A flood from an open tide floods only the dry arm.
        world.gate.tide = Tide::Flood;
        let flood = TideChange::observed(Tide::Open, Arm(2), &world.gate, 60);
        assert!(flood.flooded(2) && !flood.flooded(0) && !flood.flooded(1));
        // The tide falls back to the open arm: only it dries.
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm(2);
        let ebb = TideChange::observed(Tide::Flood, Arm(2), &world.gate, 70);
        assert!(ebb.dried(2) && !ebb.dried(0) && !ebb.dried(1));
        assert!(!ebb.flooded(0) && !ebb.flooded(1));
        assert!(!ebb.dried(7) && !ebb.flooded(7));
    }
}
