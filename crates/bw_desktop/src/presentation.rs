//! Pure presentation state helpers for the v8 art pass.
//!
//! These functions only translate already-authoritative simulation state into
//! atlas keys or deterministic cosmetic placement. They must not alter rules,
//! pathing, fog, saves, or replays.

use bw_core::{Faction, Kind, Pos, TICK_HZ, Terrain};
use bw_sim::{MapId, World};

pub const DEPLOY_TICKS: u32 = TICK_HZ as u32;
pub const FIRE_TICKS: u64 = 9;
pub const UNLOAD_TICKS: u64 = 9;
pub const SLUICE_FX_TICKS: u64 = 36;
pub const IMPACT_TICKS: u64 = 12;
pub const WRECK_TICKS: u64 = 24;
/// Quiet ground debris outlives the breakup, then clears to protect readability.
pub const RESIDUE_TICKS: u64 = 150;
pub const RESOURCE_START: u32 = 2_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactMaterial {
    Steel,
    Reed,
    Glaze,
}

impl ImpactMaterial {
    pub fn key(self) -> &'static str {
        match self {
            Self::Steel => "steel",
            Self::Reed => "reed",
            Self::Glaze => "glaze",
        }
    }
}

pub fn material_for_faction(faction: Faction) -> ImpactMaterial {
    match faction {
        Faction::Union => ImpactMaterial::Steel,
        Faction::Assembly => ImpactMaterial::Reed,
        Faction::Compact => ImpactMaterial::Glaze,
    }
}

pub fn resource_stage(remaining: u32) -> u8 {
    if remaining == 0 {
        return 3;
    }
    // Stage 0 is the intact source drawing. The three later stages are broad
    // enough to read at native scale and never expose an exact hidden amount.
    if remaining > RESOURCE_START / 2 {
        0
    } else if remaining > RESOURCE_START / 4 {
        1
    } else {
        2
    }
}

pub fn salvage_stage_asset(base: &'static str, stage: u8) -> String {
    if stage == 0 {
        base.to_string()
    } else {
        format!("{base}_stage_{stage}")
    }
}

/// Whether the cell at (x, y) holds water now: a Salter's crust is dry.
pub fn cell_is_wet_at(world: &World, x: i32, y: i32) -> bool {
    matches!(
        world.depth_at(x, y),
        Some(bw_sim::Depth::Shallow | bw_sim::Depth::Deep)
    )
}

/// Whether a tidal cell holds water now: shallow or deep.
pub fn cell_is_wet(world: &World, terrain: Terrain) -> bool {
    matches!(
        world.depth(terrain),
        Some(bw_sim::Depth::Shallow | bw_sim::Depth::Deep)
    )
}

/// The sluice art for the dry arm.  The art has two shafts, north and
/// south; arm 0 shows the north shaft dry (the Split Basin's north, and the
/// Confluence's E, whose arm also leaves the station up and to the right on
/// screen) and every other arm the south shaft (the Confluence's W and S).
/// Callers pass `gate.north_dry()`, which is exactly "arm 0 is dry".
pub fn gate_asset(north_dry: bool, warning: bool, phase: u8) -> String {
    let current = if north_dry { "north_dry" } else { "south_dry" };
    if warning {
        format!("gate_{current}_warning_{}", phase.min(3))
    } else {
        format!("gate_{current}")
    }
}

/// Ticks a frame of the lock switch shows, and its frames.
pub const GATE_SWITCH_FRAME_TICKS: u64 = 5;
pub const GATE_SWITCH_FRAMES: u64 = 12;

/// The switch frame for a lock turning so that the north shaft is dry
/// (`north_dry`) or the south, `tick` after it changed at `start`; None once
/// it rests.  Two seconds, the same as the lane's flood and drain fronts.
pub fn gate_switch_asset(north_dry: bool, start: u64, tick: u64) -> Option<String> {
    let frame = tick.checked_sub(start)? / GATE_SWITCH_FRAME_TICKS;
    let side = if north_dry { "north" } else { "south" };
    (frame < GATE_SWITCH_FRAMES).then(|| format!("gate_to_{side}_dry_{frame}"))
}

pub fn deployment_stage(entity: &bw_sim::Entity) -> Option<u8> {
    if !matches!(
        entity.kind,
        Kind::Bulwark | Kind::Loom | Kind::Heliostat | Kind::Pan
    ) || entity.deploy_remaining == 0
    {
        return None;
    }
    let stage = if entity.deploy_target {
        (DEPLOY_TICKS.saturating_sub(entity.deploy_remaining) * 3 / DEPLOY_TICKS).min(2)
    } else {
        (entity.deploy_remaining.saturating_sub(1) * 3 / DEPLOY_TICKS).min(2)
    };
    Some(stage as u8)
}

pub fn fire_phase(start_tick: u64, tick: u64) -> Option<u8> {
    let elapsed = tick.checked_sub(start_tick)?;
    let phase = match elapsed {
        0..2 => 0,
        2..5 => 1,
        5..FIRE_TICKS => 2,
        _ => return None,
    };
    Some(phase)
}

pub fn unload_phase(start_tick: u64, tick: u64) -> Option<u8> {
    let elapsed = tick.checked_sub(start_tick)?;
    (elapsed < UNLOAD_TICKS).then(|| (elapsed / 3).min(2) as u8)
}

pub fn gate_warning_phase(warning_until: Option<u64>, tick: u64) -> Option<u8> {
    let until = warning_until?;
    let total = u64::from(bw_sim::GATE_WARNING_TICKS);
    let elapsed = total.saturating_sub(until.saturating_sub(tick));
    Some(((elapsed / 6) % 4) as u8)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landmark {
    pub key: &'static str,
    pub pos: Pos,
}

/// The Split Basin's landmarks.  Positions are on existing salt approaches
/// and outside the central lanes.  They are presentation props only; no map
/// tile or collision is changed.
pub const LANDMARKS: [Landmark; 3] = [
    Landmark {
        key: "landmark_tide_gauge",
        pos: Pos::cell(45, 44),
    },
    Landmark {
        key: "landmark_ferry_stairs",
        pos: Pos::cell(45, 88),
    },
    Landmark {
        key: "landmark_hull_ribs",
        pos: Pos::cell(96, 64),
    },
];

/// The landmarks of a map.  The Split Basin's cells are other ground on the
/// Confluence (the first stands on seat T's headquarters there), so each
/// map has its own.
pub fn landmarks(map: MapId) -> &'static [Landmark] {
    match map {
        MapId::SplitBasin => &LANDMARKS,
        MapId::Confluence => &CONFLUENCE_LANDMARKS,
    }
}

/// The Confluence's landmarks: one on a bank of each arm, halfway from the
/// station to the sea and turned with the map's three-way symmetry (the tide
/// gauge on seat T's bank of the E arm, the hull ribs on seat R's bank of the
/// S arm, the ferry stairs on seat L's bank of the W arm).  Each stands on a
/// five-by-five patch of salt, well clear of the lanes, the mouths, the
/// headquarters, the wells, the wreck beds and the practice AI's sites.
pub const CONFLUENCE_LANDMARKS: [Landmark; 3] = [
    Landmark {
        key: "landmark_tide_gauge",
        pos: Pos::cell(87, 37),
    },
    Landmark {
        key: "landmark_hull_ribs",
        pos: Pos::cell(135, 113),
    },
    Landmark {
        key: "landmark_ferry_stairs",
        pos: Pos::cell(43, 113),
    },
];

/// A submerged silhouette of the drowned town, anchored at a deep-water cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submerged {
    pub key: &'static str,
    pub cell: (i32, i32),
}

/// Fixed positions inside the central deep cut, each with deep water for two
/// cells either side and one cell above and below.  Decoration only: deep
/// water stays impassable and nothing here implies a route.
pub const SUBMERGED: [Submerged; 8] = [
    Submerged {
        key: "submerged_wall",
        cell: (58, 30),
    },
    Submerged {
        key: "submerged_roof",
        cell: (68, 22),
    },
    Submerged {
        key: "submerged_hull",
        cell: (62, 40),
    },
    Submerged {
        key: "submerged_posts",
        cell: (56, 60),
    },
    Submerged {
        key: "submerged_stair",
        cell: (70, 70),
    },
    Submerged {
        key: "submerged_ring",
        cell: (64, 92),
    },
    Submerged {
        key: "submerged_wall",
        cell: (60, 100),
    },
    Submerged {
        key: "submerged_roof",
        cell: (70, 88),
    },
];

pub fn wreck_asset(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "wreck_union",
        Faction::Assembly => "wreck_assembly",
        Faction::Compact => "wreck_compact",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_stage_is_conservative_and_monotonic() {
        assert_eq!(resource_stage(RESOURCE_START), 0);
        assert_eq!(resource_stage(RESOURCE_START / 2 + 1), 0);
        assert_eq!(resource_stage(RESOURCE_START / 2), 1);
        assert_eq!(resource_stage(RESOURCE_START / 4), 2);
        assert_eq!(resource_stage(0), 3);
        for pair in [
            (2_400, 0),
            (1_801, 1_800),
            (1_200, 1_199),
            (600, 599),
            (1, 0),
        ] {
            assert!(resource_stage(pair.0) <= resource_stage(pair.1));
        }
    }

    #[test]
    fn confluence_landmarks_stand_on_quiet_salt() {
        let world = World::with_map(
            1,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .unwrap();
        assert_eq!(landmarks(MapId::SplitBasin), &LANDMARKS);
        let layout = world.map.layout();
        let mut busy: Vec<Pos> = world.map.wells.clone();
        busy.extend(world.map.resources.iter().map(|r| r.pos));
        busy.extend(world.entities.iter().map(|e| e.pos));
        for arm in layout.arms {
            busy.extend(arm.mouths.iter().map(|&(x, y)| Pos::cell(x, y)));
        }
        for seat in layout.seats {
            let ai = &seat.ai;
            busy.extend(
                [ai.works, ai.dropoff, ai.condenser]
                    .into_iter()
                    .chain(ai.drydocks)
                    .map(|(x, y)| Pos::cell(x, y)),
            );
        }
        for landmark in landmarks(MapId::Confluence) {
            let (x, y) = landmark.pos.cell_xy();
            for dy in -2..=2 {
                for dx in -2..=2 {
                    assert!(
                        matches!(
                            world.map.terrain(x + dx, y + dy),
                            Terrain::Salt | Terrain::Silt
                        ) && !layout.island_cell(x + dx, y + dy),
                        "{} at ({x},{y})",
                        landmark.key
                    );
                }
            }
            for dy in -6..=6 {
                for dx in -6..=6 {
                    assert!(
                        !world.map.terrain(x + dx, y + dy).is_tidal(),
                        "{}",
                        landmark.key
                    );
                }
            }
            for pos in &busy {
                assert!(
                    landmark.pos.distance_sq(*pos) >= i64::from(bw_core::FP * 8).pow(2),
                    "{} is within eight cells of {:?}",
                    landmark.key,
                    pos.cell_xy()
                );
            }
        }
    }

    #[test]
    fn gate_and_lane_helpers_show_current_state_during_warning() {
        let mut world = World::new(1, bw_core::Faction::Union);
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(true);
        assert!(cell_is_wet(&world, Terrain::Lane1));
        assert!(!cell_is_wet(&world, Terrain::Lane0));
        world.gate.tide = bw_sim::Tide::Neutral;
        assert!(
            cell_is_wet(&world, Terrain::Lane0),
            "shallow at the neutral tide"
        );
        assert_eq!(gate_asset(true, false, 0), "gate_north_dry");
        assert_eq!(
            gate_switch_asset(false, 100, 100).as_deref(),
            Some("gate_to_south_dry_0")
        );
        assert_eq!(
            gate_switch_asset(true, 100, 159).as_deref(),
            Some("gate_to_north_dry_11")
        );
        assert_eq!(gate_switch_asset(true, 100, 160), None);
        assert_eq!(gate_switch_asset(true, 100, 99), None);
        assert_eq!(gate_asset(true, true, 4), "gate_north_dry_warning_3");
        assert_eq!(gate_warning_phase(Some(300), 0), Some(0));
        assert_eq!(gate_warning_phase(Some(1), 300), Some(2));
    }

    #[test]
    fn deployment_fire_and_unload_timing_uses_ticks() {
        let mut e = bw_sim::Entity {
            keep_deployed: false,
            beam: None,
            build_queue: Vec::new(),
            formation: bw_sim::Formation::Compact,
            surge_remaining: 0,
            surge_cooldown: 0,
            id: 1,
            owner: 0,
            kind: Kind::Bulwark,
            pos: Pos::cell(4, 4),
            hp: 1,
            facing: 0,
            build_remaining: 0,
            queue: Vec::new(),
            deployed: false,
            deploy_remaining: DEPLOY_TICKS,
            deploy_target: true,
            after_pack: None,
            after_build: None,
            pace: 0,
            cargo: Vec::new(),
            aboard: None,
            order: bw_sim::Order::Deploy,
            max_hp: 1,
            carried: 0,
            carried_kind: None,
            gather_ticks: 0,
            attack_cooldown: 0,
            path: Vec::new(),
            path_index: 0,
            path_target: None,
            path_lane_revision: 0,
            blocked_ticks: 0,
            waypoints: Vec::new(),
            rally: None,
            builder: None,
            last_seen: None,
            upgrade: None,
            upgrade_queue: Vec::new(),
        };
        assert_eq!(deployment_stage(&e), Some(0));
        e.deploy_remaining = 15;
        assert_eq!(deployment_stage(&e), Some(1));
        e.deploy_remaining = 1;
        assert_eq!(deployment_stage(&e), Some(2));
        e.deploy_target = false;
        e.deployed = true;
        for (remaining, stage) in [(30, 2), (21, 2), (20, 1), (11, 1), (10, 0), (1, 0)] {
            e.deploy_remaining = remaining;
            assert_eq!(deployment_stage(&e), Some(stage));
        }
        assert_eq!(fire_phase(10, 10), Some(0));
        assert_eq!(fire_phase(10, 15), Some(2));
        assert_eq!(fire_phase(10, 19), None);
        assert_eq!(unload_phase(10, 16), Some(2));
        assert_eq!(unload_phase(10, 19), None);
    }
}
