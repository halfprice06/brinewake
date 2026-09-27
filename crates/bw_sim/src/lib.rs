//! Deterministic, device-independent BRINEWAKE simulation.
//!
//! This crate owns authoritative game rules, fixed-point movement, fog of war,
//! fair practice AI, and bounded save/replay encoding. It does not read wall
//! time or presentation state.

pub use bw_content::CAPTURE_WORK as CAPTURE_TICKS;
use bw_content::{
    CAPTURE_CONTEST_RADIUS_CELLS, CAPTURE_MAX_WORKERS, CAPTURE_WORK, CREW_CAP_BASE,
    CREW_CAP_DRYDOCK, CREW_CAP_MAX, CREW_CAP_WORKS, CREW_CAP_YARD, CROSSING_HOLD_RADIUS_CELLS,
    DOCTRINE_PRESSURE, DOCTRINE_SALVAGE, DOCTRINE_TICKS, DOCTRINE_TIER2_PRESSURE,
    DOCTRINE_TIER2_SALVAGE, DOCTRINE_TIER2_TICKS, DRY_EBB_TICKS, Doctrine, FLOOD_FLOTSAM,
    FLOOD_PRESSURE, FLOOD_TICKS, FLOOD_WARNING_TICKS, GLINTER_LOOM_BONUS, HQ_SALVAGE_PER_MINUTE,
    HUNTER_LOOM_SHELL_PERCENT, KEEL_PERCENT, LANE_WRECK, LOOM_BLAST_RADIUS,
    LOOM_BUILDING_DAMAGE_PERCENT, LOOM_FIRE_REVEAL_RADIUS, LOOM_FIRE_REVEAL_TICKS, LOOM_MIN_RANGE,
    LOOM_SIEGE_RANGE_CELLS, LOOM_WINDUP_TICKS, MOUTH_SCRAP_PERCENT, MOUTH_SCRAP_RADIUS_CELLS,
    NEUTRAL_CAPTURE_PERCENT, OVERFLOW_PRESSURE, OVERFLOW_SALVAGE, OVERHAUL_HULL_PERCENT,
    PRESSURE_CAP_BASE, PRESSURE_CAP_CONDENSER, PRESSURE_CAP_MAX, PRESSURE_CAP_OVERPRESSURE,
    RECLAIM_PRESSURE, RECLAIM_SALVAGE, RECYCLE_REFUND_PERCENT, REFIT_HULL_PERCENT,
    REPAIR_AURA_HULL_PER_SALVAGE, REPAIR_AURA_HULL_PER_SECOND, REPAIR_AURA_RADIUS,
    RIVETER_BUILDING_DAMAGE_PERCENT, SCOUT_HOLD_SIGHT_PERCENT, SCRAP_BASE_PERCENT,
    SCRAP_UPGRADED_PERCENT, SLUICE_PRESSURE_PER_MINUTE, SOUND_PRESSURE, SOUND_RADIUS, SOUND_TICKS,
    SOUNDER_LOOM_BONUS, STATION_SALVAGE_PER_MINUTE, SURGE_COOLDOWN_TICKS, SURGE_PRESSURE,
    SURGE_TICKS, SWAMPED_DAMAGE_PERCENT, SWITCH_PRESSURE, TEMPER_DAMAGE_PERCENT,
    TIDE_HOLD_AFTER_OUT_TICKS, TIDE_HOLD_DRAIN_PER_TICK, TIDE_HOLD_THREE_SEAT_TICKS,
    TIDE_HOLD_TICKS, UPGRADE_QUEUE, Upgrade, VENT_COOLDOWN_PERCENT, VENT_PRESSURE, VENT_TICKS,
    WADING_DAMAGE_PERCENT, WORKER_DANGER_RADIUS_CELLS, WORKER_DANGER_TICKS, faction_allows, spec,
    trains,
};
mod maps;
mod navigation;
pub mod profile;
use profile::system;
#[cfg(test)]
mod rules20;
#[cfg(test)]
mod rules21;
#[cfg(test)]
mod rules22;
#[cfg(test)]
mod trial10_orders;

pub use maps::{AiSites, ArmLayout, Layout, MapId, SeatLayout};

use bw_core::{FP, Faction, Kind, Movement, Pos, TICK_HZ, Terrain};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SIM_VERSION: u32 = 2;
/// The Split Basin's width and height, in cells. Other maps give their own.
pub const MAP_SIZE: u16 = 128;
pub const MAX_ENTITIES: usize = 1024;
/// Seats a match may hold. The Split Basin holds two and the Confluence three.
pub const MAX_SEATS: usize = 4;
pub const MAX_COMMANDS: usize = 100_000;
pub const MAX_QUEUE: usize = 16;
pub const MAX_PATH: usize = 4_096;
pub const MAX_WAYPOINTS: usize = 128;
pub const MAX_SAVE_BYTES: usize = 16 * 1024 * 1024;
/// The Split Basin's crossing mouths: the bank mouths of the two lanes,
/// north then south, west then east. A match's own map gives them as
/// `Layout::mouths`; this stays for fixtures set on the Split Basin.
pub const CROSSING_MOUTHS: [[Pos; 2]; 2] = [
    [Pos::cell(50, 49), Pos::cell(77, 49)],
    [Pos::cell(50, 79), Pos::cell(77, 79)],
];
/// The transport is the last role out: it only goes on the queue while
/// salvage is not what the army is waiting on.
const AI_TRANSPORT_RESERVE: u32 = 80;
/// How many of each Drydock role the practice AI wants, in the order
/// `Faction::drydock_roles` lists them: scout, mender, water role, transport.
const AI_DRYDOCK_ROLE_WANTED: [usize; 4] = [1, 1, 2, 1];
/// Salvage below which the practice AI turns spare pressure into more.
const AI_RECLAIM_FLOOR: u32 = 300;
/// Workers the practice AI keeps on the wrecks.
const AI_WORKER_TARGET: usize = 12;
/// How close a machine stands to count as garrisoning a mouth, in cells.
const AI_GARRISON_CELLS: i32 = 3;
/// Machines the practice AI keeps at each crossing mouth on its own bank,
/// and the force it wants before it doubles up there.
const AI_MOUTH_GARRISON: usize = 2;
const AI_HEAVY_GARRISON: usize = 9;
/// The sluice is cheap to take and pays for itself in pressure, so one
/// machine goes for it early; the mouths wait for a force worth splitting.
const AI_STATION_MINIMUM: usize = 2;
const AI_TIDE_MINIMUM: usize = 4;
/// With three seats, how near a far mouth another seat's holder denies it
/// in the practice AI's reckoning.
const AI_DENIAL_CELLS: i32 = 6;
/// How far from a mouth a force counts as staged for a crossing, in cells.
const AI_STAGING_CELLS: i32 = 8;
/// Enemy gun machines wading in the lanes before the practice AI spends a
/// flood on swamping them (rules 20: a flood no longer guards a count).
const AI_FLOOD_CATCH: usize = 3;
/// How far the transport calls for a landing party, in cells, and how long
/// it may be stuck before it puts the party down where it is.
const AI_FERRY_CALL_CELLS: i32 = 12;
const AI_FERRY_GIVE_UP: u32 = 10 * TICK_HZ as u32;
/// Wrecks on the map after this many are not added by scrap recovery.
const MAX_RESOURCES: usize = 64;
/// Sixteen-cell regions for the enemy-seen report.
const REPORT_REGION_CELLS: i32 = 16;
const REPORT_QUIET_TICKS: u64 = 20 * 30;
pub const GATE_WARNING_TICKS: u32 = 10 * TICK_HZ as u32;
pub const GATE_LOCK_TICKS: u32 = 45 * TICK_HZ as u32;
const MAX_REPLAY_TICKS: u64 = 30 * 60 * 60;
const MAX_REPAIR_WORKERS: usize = 2;
/// A new worker with no rally, or a worker whose wreck ran dry, joins the
/// nearest wreck that still holds salvage within this many cells.  The
/// reach covers a side's own wreck beds from its headquarters (the side
/// beds lie 28 cells out) and never crosses the basin (49 cells).
const AUTO_GATHER_RADIUS: i32 = 40;
/// How far a new combat machine with no rally walks from its door.
const MUSTER_CELLS: i32 = 3;
/// A rally point this close to a wreck sends new workers to gather there.
const RALLY_WRECK_RADIUS: i32 = 2;
const MAX_SELECTION: usize = 128;
const MAX_ARTILLERY_SHOTS: usize = MAX_ENTITIES;
const MAX_DANGER_MARKS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Salvage,
    Pressure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    pub id: u32,
    pub pos: Pos,
    pub remaining: u32,
    pub kind: ResourceKind,
    /// Wreckage a fight left, rather than one of the map's own beds.  The
    /// map's beds are public geography; scrap is a thing you saw happen.
    #[serde(default)]
    pub scrap: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<Terrain>,
    pub resources: Vec<Resource>,
    pub wells: Vec<Pos>,
    pub gate_pos: Pos,
    /// Which map this is. Left out on disk for the Split Basin, which every
    /// save and replay before the Confluence was played on.
    #[serde(default, skip_serializing_if = "MapId::is_split_basin")]
    pub id: MapId,
}

impl Map {
    /// The map's fixed geography: seats, arms, mouths and sites.
    pub fn layout(&self) -> &'static Layout {
        self.id.layout()
    }

    /// Whether a cell is the station's island, where nothing is built.
    pub fn island_cell(&self, x: i32, y: i32) -> bool {
        self.layout().island_cell(x, y)
    }

    pub fn terrain(&self, x: i32, y: i32) -> Terrain {
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.height) {
            return Terrain::Rock;
        }
        self.tiles[y as usize * self.width as usize + x as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    pub owner: Option<u8>,
    /// The arm that is dry once the tide is set. Written as `north_dry`, the
    /// Split Basin's name for it: see `Arm`.
    #[serde(rename = "north_dry")]
    pub dry_arm: Arm,
    pub warning_until: Option<u64>,
    pub locked_until: u64,
    pub capture_player: Option<u8>,
    pub capture_progress: u32,
    pub capture_missing_ticks: u32,
    pub transition_until: Option<u64>,
    #[serde(rename = "switch_target_north_dry")]
    pub switch_target: Option<Arm>,
    pub lane_revision: u32,
    /// The tide: neutral at the start, open on one side, or a flood.
    #[serde(default)]
    pub tide: Tide,
    /// A FLOOD waits on the warning like a switch does.
    #[serde(default)]
    pub flood_pending: bool,
    /// The tick a running flood falls back.
    #[serde(default)]
    pub flood_until: Option<u64>,
    /// Whether the tide has ever been set: a flood falls back to open.
    #[serde(default)]
    pub opened: bool,
    /// On a map of three arms, the tick the dry arm ebbs back to the
    /// neutral tide (rules 22): `DRY_EBB_TICKS` after a DRY lands. Never
    /// written while unset, so a Split Basin world hashes as it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebb_at: Option<u64>,
    /// The running warning is the ebb's (rules 22), not a switch's or a
    /// flood's: the tide goes back to neutral when it ends.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ebb_pending: bool,
}

/// An arm of tidal water, numbered by the map: on the Split Basin arm 0 is
/// the north and arm 1 the south. On disk arms 0 and 1 are written as the
/// booleans the Split Basin used for them (`true` for the north), so every
/// two-arm save and replay keeps its bytes and its state hash; a third arm is
/// written as its number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Arm(pub u8);

impl Arm {
    pub const NORTH: Arm = Arm(0);
    pub const SOUTH: Arm = Arm(1);

    /// The Split Basin's north (`true`) or south (`false`) arm.
    pub fn from_north(north: bool) -> Arm {
        if north { Arm::NORTH } else { Arm::SOUTH }
    }

    /// Whether this is the Split Basin's north arm (arm 0).
    pub fn is_north(self) -> bool {
        self == Arm::NORTH
    }

    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl Serialize for Arm {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            0 => serializer.serialize_bool(true),
            1 => serializer.serialize_bool(false),
            arm => serializer.serialize_u8(arm),
        }
    }
}

impl<'de> Deserialize<'de> for Arm {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            North(bool),
            Arm(u8),
        }
        Ok(match Written::deserialize(deserializer)? {
            Written::North(north) => Arm::from_north(north),
            Written::Arm(arm) => Arm(arm),
        })
    }
}

impl Gate {
    /// Whether arm 0, the Split Basin's north, is the dry one.
    pub fn north_dry(&self) -> bool {
        self.dry_arm.is_north()
    }
}

/// The tide's setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tide {
    /// Every arm shallow: the start of a match.
    #[default]
    Neutral,
    /// One arm dry, the others deep; `Gate::dry_arm` says which.
    Open,
    /// Every tidal cell deep.
    Flood,
}

/// The water on a tidal cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Dry,
    Shallow,
    Deep,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub faction: Faction,
    pub salvage: u32,
    pub pressure: u32,
    pub crew: u32,
    pub cap: u32,
    pub pressure_remainder: u32,
    pub salvage_spent: u64,
    pub pressure_spent: u64,
    pub doctrine: Option<Doctrine>,
    pub research: Option<Research>,
    /// 0 without a doctrine, 1 after the first tier, 2 after the second.
    #[serde(default)]
    pub doctrine_tier: u8,
    /// Completed building upgrades, sorted.
    #[serde(default)]
    pub upgrades: Vec<Upgrade>,
    /// Ticks left of the headquarters' vent: every combat machine fires faster.
    #[serde(default)]
    pub vent_remaining: u32,
    /// The pressure cap, from condensers and Overpressure.
    #[serde(default = "default_pressure_cap")]
    pub pressure_cap: u32,
    /// Where this side's workers were hit lately (rules 13).
    #[serde(default)]
    pub worker_danger: Vec<DangerMark>,
    /// Pressure income that arrived over the cap and has not yet become
    /// salvage (rules 15).
    #[serde(default)]
    pub overflow: u32,
    /// Pan pressure not yet a whole point, in pressure-ticks (rules 18).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pan_remainder: u32,
    /// Headquarters and station salvage not yet a whole point, in
    /// salvage-ticks (rules 20).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub salvage_remainder: u32,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// A save from before the hold gauge was stored holds two seats at zero.
fn two_seat_gauges() -> Vec<u32> {
    vec![0, 0]
}

fn default_pressure_cap() -> u32 {
    PRESSURE_CAP_BASE
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Research {
    pub building: u32,
    pub doctrine: Doctrine,
    pub remaining: u32,
    /// 1 for the first tier, 2 for the second.
    #[serde(default = "default_tier")]
    pub tier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Production {
    pub kind: Kind,
    pub remaining: u32,
    pub started: bool,
    pub cost_salvage: u32,
    pub cost_pressure: u32,
}

fn default_tier() -> u8 {
    1
}

/// A building upgrade in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeJob {
    pub upgrade: Upgrade,
    pub remaining: u32,
}

/// A patch of borrowed sight for its owner around a point until a tick: a
/// SOUND ping, or a firing Loom lit for the side its shell falls on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beacon {
    pub owner: u8,
    pub pos: Pos,
    pub until: u64,
    /// How far the light reaches, in fixed-point units.
    #[serde(default = "default_beacon_radius")]
    pub radius: i32,
    /// GLINT's ray: the light runs from `pos` to here, `radius` either
    /// side of it (rules 18). Without it the light is a circle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ray_to: Option<Pos>,
}

impl Beacon {
    /// Whether the light reaches `point`.
    pub fn lights(&self, point: Pos) -> bool {
        let reach = i64::from(self.radius).pow(2);
        let Some(end) = self.ray_to else {
            return self.pos.distance_sq(point) <= reach;
        };
        // Distance to the segment, in exact integers: project the point
        // onto the ray and clamp to its ends.
        let (ax, ay) = (i64::from(self.pos.x), i64::from(self.pos.y));
        let (dx, dy) = (i64::from(end.x) - ax, i64::from(end.y) - ay);
        let (px, py) = (i64::from(point.x) - ax, i64::from(point.y) - ay);
        let length_sq = dx * dx + dy * dy;
        if length_sq == 0 {
            return px * px + py * py <= reach;
        }
        let along = (px * dx + py * dy).clamp(0, length_sq);
        // |p - d * along / length_sq|^2, scaled by length_sq^2.
        let ex = px * length_sq - dx * along;
        let ey = py * length_sq - dy * along;
        let scaled = i128::from(ex) * i128::from(ex) + i128::from(ey) * i128::from(ey);
        scaled <= i128::from(reach) * i128::from(length_sq) * i128::from(length_sq)
    }
}

fn default_beacon_radius() -> i32 {
    SOUND_RADIUS
}

/// Ground where a worker of `Player` was hit: gatherers leave wrecks inside
/// it and no worker picks one on its own until `until`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DangerMark {
    pub pos: Pos,
    pub until: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Order {
    #[default]
    Idle,
    Move {
        target: Pos,
    },
    AttackMove {
        target: Pos,
    },
    Attack {
        target: u32,
    },
    Gather {
        resource: u32,
    },
    Build {
        target: u32,
    },
    Repair {
        target: u32,
    },
    Capture,
    Hold,
    Deploy,
    /// Walk to a transport and climb aboard.
    Board {
        transport: u32,
    },
    /// A Salter's LAY (rules 18): it stands on `head`, the last dry cell
    /// of its causeway, and crusts the next row along (`dx`, `dy`) every
    /// LAY_ROW_TICKS. `rows` counts the rows laid so far.
    Lay {
        head: Pos,
        dx: i8,
        dy: i8,
        rows: u16,
        progress: u16,
    },
    /// A worker walking to an own headquarters, Works or Salvage Yard to be
    /// broken up for part of its cost (rules 19).
    Recycle {
        target: u32,
    },
    /// A worker taking its load to an own headquarters or Salvage Yard
    /// (rules 22): it hands the load in and goes back to `resource`, the
    /// wreck it came from, once that wreck is safe again, or to the nearest
    /// safe wreck; with none it stands idle. Workers pulled back from a
    /// wreck under fire take this order, and so does DELIVER.
    Deliver {
        target: u32,
        resource: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: u32,
    pub owner: u8,
    pub kind: Kind,
    pub pos: Pos,
    pub hp: i32,
    pub facing: u8,
    pub build_remaining: u32,
    pub queue: Vec<Production>,
    pub deployed: bool,
    pub deploy_remaining: u32,
    pub deploy_target: bool,
    pub formation: Formation,
    pub surge_remaining: u32,
    pub surge_cooldown: u32,
    pub order: Order,
    pub max_hp: i32,
    pub carried: u32,
    pub carried_kind: Option<ResourceKind>,
    pub gather_ticks: u32,
    pub attack_cooldown: u32,
    pub path: Vec<Pos>,
    pub path_index: usize,
    pub path_target: Option<Pos>,
    pub path_lane_revision: u32,
    pub blocked_ticks: u32,
    pub waypoints: Vec<Pos>,
    pub rally: Option<Pos>,
    pub builder: Option<u32>,
    pub last_seen: Option<u64>,
    /// A building's running upgrade.
    #[serde(default)]
    pub upgrade: Option<UpgradeJob>,
    /// Upgrades paid for and waiting behind the running one (rules 15).
    #[serde(default)]
    pub upgrade_queue: Vec<Upgrade>,
    /// The order a packing specialist takes up once its legs are free: a
    /// move given to a deployed machine packs it first and then sends it.
    #[serde(default)]
    pub after_pack: Option<Order>,
    /// The pace an attack-move holds a group to: the slowest member's speed,
    /// so a mixed group arrives together instead of feeding its fastest
    /// machines to the enemy first.  Zero is full speed.
    #[serde(default)]
    pub pace: i32,
    /// A transport's hold: the machines aboard, in boarding order.
    #[serde(default)]
    pub cargo: Vec<u32>,
    /// The transport this machine rides in; it is not on the field then.
    #[serde(default)]
    pub aboard: Option<u32>,
    /// A deployed specialist told to stay deployed: a move or attack-move
    /// given to its group leaves it where it stands (rules 14).
    #[serde(default)]
    pub keep_deployed: bool,
    /// A worker's next sites, in order: each is taken up when the one it is
    /// building is finished (rules 14).
    #[serde(default)]
    pub build_queue: Vec<u32>,
    /// The order a worker takes up once its sites are built: a shift-queued
    /// gather or move no longer replaces the site it is on (rules 19).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_build: Option<Order>,
    /// A Heliostat's beam: the target it has been burning and how hard the
    /// last shot hit (rules 18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam: Option<Beam>,
}

/// The build-up of a Heliostat's beam on one target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beam {
    pub target: u32,
    /// The last shot's damage, before upgrades and modifiers.
    pub damage: i32,
    pub last_tick: u64,
}

/// Who is building an unfinished site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteBuilder {
    /// Its worker is on it or walking to it.
    Building,
    /// Its worker will come once it finishes the site it is on.
    Queued,
    /// Nobody: a worker must be sent to resume it.
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Formation {
    #[default]
    Compact,
    Line,
    Loose,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtilleryShot {
    pub owner: u8,
    pub source: u32,
    pub from: Pos,
    pub target: Pos,
    pub impact_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Move {
        units: Vec<u32>,
        target: Pos,
        queued: bool,
    },
    AttackMove {
        units: Vec<u32>,
        target: Pos,
        queued: bool,
    },
    Attack {
        units: Vec<u32>,
        target: u32,
    },
    Gather {
        units: Vec<u32>,
        resource: u32,
    },
    Build {
        worker: u32,
        kind: Kind,
        pos: Pos,
        /// Build after the sites the worker already has (rules 14): a
        /// worker given a second site no longer walks off its first.
        #[serde(default)]
        queued: bool,
    },
    Train {
        building: u32,
        kind: Kind,
    },
    Cancel {
        building: u32,
    },
    Rally {
        building: u32,
        pos: Pos,
    },
    Stop {
        units: Vec<u32>,
    },
    Hold {
        units: Vec<u32>,
    },
    Deploy {
        units: Vec<u32>,
    },
    Capture {
        units: Vec<u32>,
    },
    /// Deploy or pack, never the other way (rules 14): `Deploy` toggles,
    /// and a second press mid-pack turned a line the wrong way.
    SetDeployed {
        units: Vec<u32>,
        deployed: bool,
    },
    /// A deployed specialist that keeps deployed ignores move and
    /// attack-move orders given to its group and stays where it stands
    /// until it is told to pack (rules 14).
    KeepDeployed {
        units: Vec<u32>,
        keep: bool,
    },
    /// Open the next arm (arm 0, the north, from the neutral tide).
    SwitchGate,
    /// Open an arm: dry there, deep on the others. Written as `north`, the
    /// Split Basin's name for it: see `Arm`.
    SetTide {
        #[serde(rename = "north")]
        arm: Arm,
    },
    /// Every tidal cell deep for a while.
    Flood,
    /// Machines walk to an own transport and board it.
    Board {
        units: Vec<u32>,
        transport: u32,
    },
    /// A transport sets its hold on free ground beside it.
    Unload {
        transport: u32,
    },
    Repair {
        units: Vec<u32>,
        target: u32,
    },
    SetFormation {
        units: Vec<u32>,
        formation: Formation,
    },
    Face {
        units: Vec<u32>,
        target: Pos,
    },
    Surge {
        units: Vec<u32>,
    },
    Research {
        building: u32,
        doctrine: Doctrine,
    },
    CancelResearch {
        building: u32,
    },
    /// A building upgrade at the building that offers it.
    Upgrade {
        building: u32,
        upgrade: Upgrade,
    },
    CancelUpgrade {
        building: u32,
    },
    /// The headquarters vents its pressure: faster fire for ten seconds.
    Vent {
        building: u32,
    },
    /// The headquarters turns pressure into salvage at once.
    Reclaim {
        building: u32,
    },
    /// A Sounder or Skipper sounds the ground: sight around it for five seconds.
    Sound {
        unit: u32,
    },
    /// A Glinter flashes GLINT: a long, thin ray of sight toward `target`
    /// for five seconds (rules 18).
    Glint {
        unit: u32,
        target: Pos,
    },
    /// A Salter lays a causeway: from the bank behind `target` toward it,
    /// across the tidal water (rules 18).
    Lay {
        unit: u32,
        target: Pos,
    },
    /// Workers walk to the nearest own headquarters, Works or Salvage Yard
    /// and are broken up there: half their salvage back and their crew
    /// place free (rules 19). A worker with nothing left to gather no
    /// longer sits on crew.
    Recycle {
        units: Vec<u32>,
    },
    Surrender,
    /// Gather once the workers' sites are built (rules 19): shift with a
    /// gather order walked a builder off its site in trial 10.  A worker
    /// with no site gathers at once.
    QueueGather {
        units: Vec<u32>,
        resource: u32,
    },
    /// Loaded workers take their load to an own headquarters or Salvage
    /// Yard and go back to the wreck they were on (rules 22). An empty
    /// worker keeps its gather.
    Deliver {
        units: Vec<u32>,
        target: u32,
    },
}

impl Command {
    fn command_target_len(&self) -> usize {
        match self {
            Self::Move { units, .. }
            | Self::AttackMove { units, .. }
            | Self::Attack { units, .. }
            | Self::Gather { units, .. }
            | Self::QueueGather { units, .. }
            | Self::Stop { units }
            | Self::Hold { units }
            | Self::Deploy { units }
            | Self::Capture { units }
            | Self::SetDeployed { units, .. }
            | Self::KeepDeployed { units, .. }
            | Self::Repair { units, .. }
            | Self::SetFormation { units, .. }
            | Self::Face { units, .. }
            | Self::Surge { units }
            | Self::Deliver { units, .. }
            | Self::Recycle { units } => units.len(),
            Self::Sound { .. } | Self::Glint { .. } | Self::Lay { .. } => 1,
            Self::Build { .. }
            | Self::Train { .. }
            | Self::Cancel { .. }
            | Self::Rally { .. }
            | Self::Research { .. }
            | Self::CancelResearch { .. }
            | Self::Upgrade { .. }
            | Self::CancelUpgrade { .. }
            | Self::Vent { .. }
            | Self::Reclaim { .. }
            | Self::SwitchGate
            | Self::SetTide { .. }
            | Self::Flood
            | Self::Board { .. }
            | Self::Unload { .. }
            | Self::Surrender => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Victory(u8),
    Draw,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    CommandAccepted,
    CommandRejected,
    GateCaptureStarted,
    GateCaptured,
    GateWarning,
    GateChanged,
    GateLocked,
    Shot,
    ArtilleryWarning,
    Damage,
    Death,
    Gather,
    Deposit,
    BuildStarted,
    BuildCompleted,
    ProductionQueued,
    ProductionCompleted,
    Repair,
    Deploy,
    ResearchStarted,
    ResearchCompleted,
    ResearchCancelled,
    Surge,
    Victory,
    Draw,
    AiOrder,
    UpgradeStarted,
    UpgradeQueued,
    UpgradeCompleted,
    UpgradeCancelled,
    Vent,
    Sound,
    /// An enemy combat machine seen in a region after twenty quiet seconds;
    /// `other` is 1 when the region holds one of the player's buildings.
    EnemySeen,
    SwitchCancelled,
    /// A machine found no way to its destination and stopped.
    PathBlocked,
    /// A ground machine caught by the rising tide, sent to the shore.
    Swamped,
    /// A machine climbed aboard a transport.
    Boarded,
    /// A machine was set down by its transport.
    Unloaded,
    /// The headquarters turned pressure into salvage; `amount` is the salvage.
    Reclaimed,
    /// A wreck gave its last load; `other` is the wreck, `from` its cell and
    /// the text says whether the workers find another.
    WreckEmptied,
    /// Workers hit under fire left their wrecks (rules 13); `amount` is how
    /// many and `from` where the first was hit.
    WorkersFled,
    /// A seat was knocked out of a match of three or more: its headquarters
    /// fell or it surrendered. `player` is the seat.
    Eliminated,
    /// A Glinter flashed GLINT from `from` toward `to` (rules 18).
    Glint,
    /// A Salter crusted a row of its causeway at `to` (rules 18).
    Laid,
    /// The tide changed and the crust melted; `amount` is how many cells.
    CrustMelted,
    /// A worker was broken up at an own headquarters, Works or Salvage Yard
    /// (rules 19): `entity` is the worker, `amount` the salvage returned
    /// and `from` where it stood. Its crew place is free again.
    Recycled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub tick: u64,
    pub kind: EventKind,
    pub player: Option<u8>,
    pub entity: Option<u32>,
    pub other: Option<u32>,
    pub from: Option<Pos>,
    pub to: Option<Pos>,
    pub amount: i32,
    pub text: String,
    /// For a Death: the kind of what did it, fixed before the killer may
    /// fall too.
    #[serde(default)]
    pub cause: Option<Kind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub tick: u64,
    pub player: u8,
    pub sequence: u64,
    pub command: Command,
    pub accepted: bool,
    pub applied: Option<bool>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEntity {
    pub id: u32,
    pub owner: u8,
    pub kind: Kind,
    pub pos: Pos,
    pub hp: Option<i32>,
    pub last_seen: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerKnowledge {
    pub player: u8,
    pub tick: u64,
    pub own: Vec<KnowledgeEntity>,
    pub visible: Vec<KnowledgeEntity>,
    pub stale: Vec<KnowledgeEntity>,
    pub gate: Gate,
    pub map: Map,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ScheduledCommand {
    execute_tick: u64,
    player: u8,
    sequence: u64,
    command: Command,
    log_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct InitialState {
    seed: u64,
    map: Map,
    gate: Gate,
    entities: Vec<Entity>,
    players: Vec<Player>,
    next_entity: u32,
    rng_state: u64,
}

/// How hard the practice AI plays. Normal is the AI every earlier match
/// and replay was recorded with, and is never written, so their saves,
/// replays and hashes are unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiLevel {
    /// Runs a smaller economy (seven workers, not twelve), thinks half as
    /// often and sends no attack before eight minutes.
    Easy,
    #[default]
    Normal,
}

impl AiLevel {
    pub const ALL: [AiLevel; 2] = [AiLevel::Easy, AiLevel::Normal];

    pub fn is_normal(&self) -> bool {
        *self == AiLevel::Normal
    }

    /// Ticks between the practice AI's turns.
    fn interval(self) -> u64 {
        match self {
            AiLevel::Easy => 30,
            AiLevel::Normal => 15,
        }
    }

    /// The first tick the practice AI's army may leave home.
    fn first_attack(self) -> u64 {
        match self {
            AiLevel::Easy => 8 * 60 * TICK_HZ,
            AiLevel::Normal => 450,
        }
    }

    /// The workers the practice AI trains up to.
    fn worker_target(self) -> usize {
        match self {
            AiLevel::Easy => 7,
            AiLevel::Normal => AI_WORKER_TARGET,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AiLevel::Easy => "EASY",
            AiLevel::Normal => "NORMAL",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ReplayFile {
    version: u32,
    rules_digest: String,
    initial: InitialState,
    commands: Vec<CommandRecord>,
    end_tick: u64,
    ai_enabled: bool,
    ai_last_tick: u64,
    #[serde(default, skip_serializing_if = "AiLevel::is_normal")]
    ai_level: AiLevel,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SaveEnvelope {
    version: u32,
    rules_digest: String,
    state_hash: String,
    payload_digest: String,
    world: World,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub tick: u64,
    pub entities: Vec<Entity>,
    /// One per seat, in seat order. Two on the Split Basin.
    pub players: Vec<Player>,
    pub map: Map,
    pub gate: Gate,
    pub artillery: Vec<ArtilleryShot>,
    pub outcome: Option<Outcome>,
    pub events: Vec<Event>,
    pub command_log: Vec<CommandRecord>,
    pending: Vec<ScheduledCommand>,
    initial: Option<InitialState>,
    pub seed: u64,
    pub rng_state: u64,
    pub lane_revision: u32,
    pub ai_enabled: bool,
    ai_last_tick: u64,
    /// How hard the practice AI plays; set before the first step.
    #[serde(default, skip_serializing_if = "AiLevel::is_normal")]
    pub ai_level: AiLevel,
    /// A replay of a practice match: its AI is off and its orders come from
    /// the recording, but seat 1's machines still muster the way the AI's
    /// did, or the replay parts from the match it recorded.
    #[serde(skip)]
    replaying_ai: bool,
    next_entity_id: u32,
    observations: Vec<Vec<KnowledgeEntity>>,
    /// SOUND pings still lighting the ground.
    #[serde(default)]
    pub beacons: Vec<Beacon>,
    /// Tidal cells a Salter crusted into dry ground (rules 18), as sorted
    /// indices `y * width + x`. They melt when the tide next changes.
    /// Never written while empty, so a world without a Compact hashes as
    /// it did before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crust: Vec<u32>,
    /// The hold gauge per player: rises a tick for every tick both crossing
    /// mouths are held unopposed and falls while they are not.
    #[serde(default = "two_seat_gauges")]
    pub lane_hold: Vec<u32>,
    /// Per player, the regions where an enemy was last reported and when.
    /// Presentation history: never hashed, never restored.
    #[serde(skip)]
    enemy_reports: Vec<Vec<(i32, i32, u64)>>,
    /// Seats knocked out of a match of three or more, with the tick each
    /// went out, in order. Never written while empty, so a two-seat world
    /// serializes and hashes as it did before seats were counted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eliminated: Vec<(u8, u64)>,
    /// Presentation only: an observer's world with the fog lifted.  Never
    /// serialized, never hashed, never set by the simulation.
    #[serde(skip)]
    pub revealed: bool,
    /// Presentation only: how far a relabelled view has rotated the seats
    /// (the local seat's number in the match). The map's seats, arms and
    /// banks keep the match's numbers, so the seat helpers turn back
    /// through this. Never serialized, never hashed.
    #[serde(skip)]
    pub view_turn: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DamageEvent {
    attacker: u32,
    target: u32,
    amount: i32,
    from: Pos,
    to: Pos,
}

impl Player {
    fn new(faction: Faction) -> Self {
        Self {
            faction,
            salvage: 360,
            pressure: 40,
            crew: 6,
            cap: CREW_CAP_BASE,
            pressure_remainder: 0,
            salvage_spent: 0,
            pressure_spent: 0,
            doctrine: None,
            research: None,
            doctrine_tier: 0,
            upgrades: Vec::new(),
            vent_remaining: 0,
            pressure_cap: PRESSURE_CAP_BASE,
            worker_danger: Vec::new(),
            overflow: 0,
            pan_remainder: 0,
            salvage_remainder: 0,
        }
    }
}

impl Map {
    fn new(id: MapId) -> Self {
        let ground = maps::ground(id);
        let resources = ground
            .wrecks
            .iter()
            .zip(1u32..)
            .map(|(&(x, y, remaining), id)| Resource {
                scrap: false,
                id,
                pos: Pos::cell(x, y),
                remaining,
                kind: ResourceKind::Salvage,
            })
            .collect();
        Self {
            width: ground.width,
            height: ground.height,
            tiles: ground.tiles,
            resources,
            wells: ground.wells,
            gate_pos: ground.station,
            id,
        }
    }
}

impl World {
    /// A two-seat match on the Split Basin: `faction` against the other.
    pub fn new(seed: u64, faction: Faction) -> World {
        let opposite = match faction {
            Faction::Union => Faction::Assembly,
            Faction::Assembly | Faction::Compact => Faction::Union,
        };
        Self::with_factions(seed, &[faction, opposite])
            .expect("two seats always fit the Split Basin")
    }

    /// A match on the Split Basin with one seat per faction listed, in seat
    /// order. The same faction may sit in more than one seat.
    pub fn with_factions(seed: u64, factions: &[Faction]) -> Result<World, String> {
        Self::with_map(seed, MapId::SplitBasin, factions)
    }

    /// A match on `map` with one seat per faction listed, in seat order. The
    /// map sets the number of seats: two on the Split Basin, three on the
    /// Confluence.
    pub fn with_map(seed: u64, map: MapId, factions: &[Faction]) -> Result<World, String> {
        let layout = map.layout();
        if factions.len() != layout.seat_count() {
            return Err(format!(
                "{} holds {} players, not {}.",
                layout.name,
                layout.seat_count(),
                factions.len()
            ));
        }
        let seats = factions.len();
        let map = Map::new(map);
        let gate = Gate {
            owner: None,
            dry_arm: Arm(0),
            warning_until: None,
            locked_until: 0,
            capture_player: None,
            capture_progress: 0,
            capture_missing_ticks: 0,
            transition_until: None,
            switch_target: None,
            lane_revision: 0,
            tide: Tide::Neutral,
            flood_pending: false,
            flood_until: None,
            opened: false,
            ebb_at: None,
            ebb_pending: false,
        };
        let mut world = Self {
            tick: 0,
            entities: Vec::new(),
            players: factions
                .iter()
                .map(|&faction| Player::new(faction))
                .collect(),
            map,
            gate,
            artillery: Vec::new(),
            outcome: None,
            events: Vec::new(),
            command_log: Vec::new(),
            pending: Vec::new(),
            initial: None,
            seed,
            rng_state: seed | 1,
            lane_revision: 0,
            ai_enabled: true,
            ai_last_tick: 0,
            ai_level: AiLevel::Normal,
            replaying_ai: false,
            beacons: Vec::new(),
            crust: Vec::new(),
            lane_hold: vec![0; seats],
            enemy_reports: vec![Vec::new(); seats],
            next_entity_id: 1,
            observations: vec![Vec::new(); seats],
            eliminated: Vec::new(),
            revealed: false,
            view_turn: 0,
        };
        world.spawn_starting_layout();
        for player in 0..seats as u8 {
            world.assign_idle_workers(player);
        }
        if world.initial.is_none() {
            world.initial = Some(world.initial_state());
        }
        Ok(world)
    }

    /// The hold gauges as the local seat reads them: its own, and the
    /// highest of any opponent's. With two seats this is `lane_hold` itself.
    pub fn hold_pair(&self) -> [u32; 2] {
        let own = self.lane_hold.first().copied().unwrap_or(0);
        let enemy = self.lane_hold.iter().skip(1).copied().max().unwrap_or(0);
        [own, enemy]
    }

    /// The opponent of seat 0 with the highest hold gauge (the first on a
    /// tie), and that gauge.
    pub fn leading_enemy_gauge(&self) -> Option<(u8, u32)> {
        self.lane_hold
            .iter()
            .enumerate()
            .skip(1)
            .max_by_key(|&(seat, &ticks)| (ticks, std::cmp::Reverse(seat)))
            .map(|(seat, &ticks)| (seat as u8, ticks))
    }

    /// The map's number for a seat of this world: the same, unless this is
    /// a relabelled view, whose seats are rotated so the local one is 0.
    pub fn layout_seat(&self, seat: u8) -> u8 {
        let seats = self.players.len().max(1);
        ((usize::from(seat) + usize::from(self.view_turn)) % seats) as u8
    }

    /// This world's number for one of the map's seats.
    fn view_seat(&self, layout_seat: u8) -> u8 {
        let seats = self.players.len().max(1);
        let turn = usize::from(self.view_turn) % seats;
        ((usize::from(layout_seat) + seats - turn) % seats) as u8
    }

    /// The arms whose lanes touch a seat's land: the lanes it must hold for
    /// the tide victory. Both arms on the Split Basin; the two beside it on
    /// the Confluence.
    pub fn arms_of(&self, seat: u8) -> Vec<usize> {
        self.map.layout().arms_of(self.layout_seat(seat)).collect()
    }

    /// The seats an arm's lane joins, as this world numbers them, in the
    /// order of the arm's mouths.
    pub fn arm_banks(&self, arm: usize) -> [u8; 2] {
        let layout = self.map.layout();
        let banks = layout.arms[arm.min(layout.arm_count() - 1)].banks;
        [self.view_seat(banks[0]), self.view_seat(banks[1])]
    }

    /// The arm whose lane joins two seats, if one does.
    pub fn arm_between(&self, a: u8, b: u8) -> Option<usize> {
        self.map
            .layout()
            .arm_between(self.layout_seat(a), self.layout_seat(b))
    }

    /// The mouth of an arm's lane on a seat's bank, if the arm touches it.
    pub fn own_mouth(&self, seat: u8, arm: usize) -> Option<Pos> {
        let side = self.arm_banks(arm).iter().position(|&bank| bank == seat)?;
        Some(self.map.layout().arm_mouths(arm)[side])
    }

    /// Each arm's two mouths, in arm order.
    pub fn crossing_mouths(&self) -> Vec<[Pos; 2]> {
        self.map.layout().mouths().collect()
    }

    /// The lanes a seat must hold for its tide count to run: every lane
    /// that touches its land, less (rules 20) a lane whose other bank
    /// belongs to a seat that is out. Nobody standing can contest that
    /// lane, so it no longer decides the count. The Split Basin never
    /// knocks a seat out, so there it is always both arms.
    pub fn hold_arms(&self, seat: u8) -> Vec<usize> {
        self.arms_of(seat)
            .into_iter()
            .filter(|&arm| {
                self.arm_banks(arm)
                    .iter()
                    .all(|&bank| bank == seat || !self.is_eliminated(bank))
            })
            .collect()
    }

    /// Whether a seat holds every lane it must for the tide count to run.
    pub fn holds_every_lane(&self, seat: u8) -> bool {
        let arms = self.hold_arms(seat);
        !arms.is_empty() && arms.into_iter().all(|arm| self.holds_lane(seat, arm))
    }

    /// How long a count must run to win this match now (rules 20): 90 s
    /// with two seats, 75 s while three or more stand, and 120 s for the
    /// rest of the match once a seat is out.
    pub fn hold_ticks(&self) -> u32 {
        hold_ticks_for(self.players.len(), !self.eliminated.is_empty())
    }

    /// Whether every hold count is frozen (rules 20): while a flood runs
    /// no count rises or drains, and each resumes where it stopped when
    /// the flood falls. The five-second warning before it does not freeze.
    pub fn hold_frozen(&self) -> bool {
        self.gate.flood_until.is_some()
    }

    /// Where a seat's headquarters stood at the start.
    pub fn start_of(&self, seat: u8) -> Pos {
        self.map.layout().headquarters(self.layout_seat(seat))
    }

    /// How many seats play this match.
    pub fn seat_count(&self) -> usize {
        self.players.len()
    }

    /// The seats, as player numbers.
    pub fn seats(&self) -> std::ops::Range<u8> {
        0..self.players.len() as u8
    }

    /// Whether `player` has been knocked out of a match of three or more.
    pub fn is_eliminated(&self, player: u8) -> bool {
        self.eliminated.iter().any(|&(seat, _)| seat == player)
    }

    /// Rebase a clean tick-zero world after a headless fixture has adjusted
    /// public map or entity fields.  Scenario builders use this to make their
    /// authored setup the deterministic replay origin without exposing
    /// internal spawn helpers.
    pub fn reset_fixture_origin(&mut self) -> Result<(), String> {
        if self.tick != 0
            || !self.pending.is_empty()
            || !self.command_log.is_empty()
            || !self.artillery.is_empty()
            || self.outcome.is_some()
        {
            return Err("fixture origin can only reset a clean tick-zero world".to_string());
        }
        let max_id = self
            .entities
            .iter()
            .map(|entity| entity.id)
            .max()
            .unwrap_or(0);
        self.next_entity_id = self.next_entity_id.max(max_id.saturating_add(1)).max(1);
        self.events.clear();
        self.initial = Some(self.initial_state());
        self.validate_invariants()
    }

    pub fn step(&mut self) {
        if self.outcome.is_some() {
            self.events.clear();
            return;
        }
        self.events.clear();
        // Timing only, when a watchdog asks: it never touches the world.
        let mut probe = profile::Probe::start();
        system!(probe, self.apply_gate_schedule());
        system!(probe, self.run_ai());
        system!(probe, self.apply_pending_commands());
        system!(probe, self.update_caps());
        system!(probe, self.update_pressure());
        system!(probe, self.update_salvage_trickle());
        system!(probe, self.update_construction());
        system!(probe, self.update_production());
        system!(probe, self.update_research());
        system!(probe, self.update_upgrades());
        system!(probe, self.update_deployment());
        system!(probe, self.update_workers());
        system!(probe, self.update_repair_auras());
        system!(probe, self.update_fight_then_pack());
        system!(probe, self.update_artillery_stance());
        system!(probe, self.update_navigation());
        system!(probe, self.update_laying());
        system!(probe, self.update_spread());
        system!(probe, self.update_boarding());
        system!(probe, self.update_idle_engagement());
        let gatherers = system!(probe, self.gatherer_owners());
        system!(probe, self.update_combat());
        system!(probe, self.update_artillery());
        system!(probe, self.update_worker_danger(&gatherers));
        system!(probe, self.update_gate_capture());
        system!(probe, self.update_switch_guard());
        system!(probe, self.update_surge());
        system!(probe, self.update_tide_hold());
        system!(probe, self.update_victory());
        system!(probe, self.update_visibility_memory());
        system!(probe, self.update_enemy_reports());
        self.beacons.retain(|beacon| beacon.until > self.tick);
        self.tick = self.tick.saturating_add(1);
        self.events
            .retain(|event| event.tick == self.tick.saturating_sub(1));
    }

    pub fn issue(&mut self, player: u8, command: Command) -> Result<(), String> {
        if self.outcome.is_some() {
            return Err("match is already over".to_string());
        }
        if usize::from(player) >= self.players.len() {
            return Err("invalid player".to_string());
        }
        if self.is_eliminated(player) {
            return Err("you are out of this match".to_string());
        }
        if self.command_log.len() >= MAX_COMMANDS {
            return Err("command log limit reached".to_string());
        }
        let sequence = self
            .command_log
            .iter()
            .filter(|record| record.player == player)
            .map(|record| record.sequence)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if let Err(reason) = self.validate_submission(player, &command) {
            self.command_log.push(CommandRecord {
                tick: self.tick.saturating_add(1),
                player,
                sequence,
                command: command.clone(),
                accepted: false,
                applied: Some(false),
                reason: Some(reason.clone()),
            });
            self.events.push(Event::text(
                self.tick,
                EventKind::CommandRejected,
                Some(player),
                reason.clone(),
            ));
            return Err(reason);
        }
        let log_index = self.command_log.len();
        self.command_log.push(CommandRecord {
            tick: self.tick.saturating_add(1),
            player,
            sequence,
            command: command.clone(),
            accepted: true,
            applied: None,
            reason: None,
        });
        self.pending.push(ScheduledCommand {
            execute_tick: self.tick,
            player,
            sequence,
            command,
            log_index,
        });
        Ok(())
    }

    /// Validate a command for `player` against the current state without
    /// queuing it.  Lockstep peers use this for immediate feedback while the
    /// authoritative validation happens again when the command executes.
    pub fn validate(&self, player: u8, command: &Command) -> Result<(), String> {
        if self.outcome.is_some() {
            return Err("match is already over".to_string());
        }
        if usize::from(player) >= self.players.len() {
            return Err("invalid player".to_string());
        }
        if self.is_eliminated(player) {
            return Err("you are out of this match".to_string());
        }
        self.validate_submission(player, command)
    }

    /// A presentation copy of this world in which `local` is player 0.
    ///
    /// The desktop presents everything from player 0's seat.  A lockstep
    /// guest who is canonically player 1 draws and reads this relabelled
    /// copy instead: every owner, player slot, gate owner, capture, shot,
    /// event, outcome and observation swaps sides, while ids, positions and
    /// the tick stay the same, so commands built against the view are valid
    /// against the canonical world.  The copy is never stepped or hashed.
    pub fn relabeled_for(&self, local: u8) -> World {
        // Every field of the world is named here on purpose. A new one stops
        // this function compiling until someone has decided whether a seat
        // swap touches it, because the test below cannot catch a field that
        // is never swapped at all: leaving one out is still an involution.
        // Do not answer the compiler with `..`.
        let World {
            // Swapped further down.
            entities: _,
            players: _,
            gate: _,
            artillery: _,
            outcome: _,
            events: _,
            command_log: _,
            pending: _,
            observations: _,
            beacons: _,
            crust: _,
            lane_hold: _,
            enemy_reports: _,
            eliminated: _,
            initial: _,
            // The same from either seat.
            tick: _,
            map: _,
            seed: _,
            rng_state: _,
            lane_revision: _,
            ai_enabled: _,
            ai_last_tick: _,
            ai_level: _,
            replaying_ai: _,
            next_entity_id: _,
            revealed: _,
            // Set below from the rotation.
            view_turn: _,
        } = self;
        let mut view = self.clone();
        let seats = self.players.len();
        if local == 0 || usize::from(local) >= seats {
            return view;
        }
        // A rotation: the local seat becomes 0 and the others keep their
        // order after it. With two seats this is the old swap.
        let flip = |player: u8| {
            if usize::from(player) < seats {
                ((usize::from(player) + seats - usize::from(local)) % seats) as u8
            } else {
                player
            }
        };
        let turn = usize::from(local);
        view.view_turn = ((usize::from(self.view_turn) + turn) % seats) as u8;
        for entity in &mut view.entities {
            entity.owner = flip(entity.owner);
        }
        view.players.rotate_left(turn);
        view.gate.owner = view.gate.owner.map(flip);
        view.gate.capture_player = view.gate.capture_player.map(flip);
        for shot in &mut view.artillery {
            shot.owner = flip(shot.owner);
        }
        view.outcome = view.outcome.as_ref().map(|outcome| match outcome {
            Outcome::Victory(player) => Outcome::Victory(flip(*player)),
            Outcome::Draw => Outcome::Draw,
        });
        for event in &mut view.events {
            event.player = event.player.map(flip);
        }
        for record in &mut view.command_log {
            record.player = flip(record.player);
        }
        for scheduled in &mut view.pending {
            scheduled.player = flip(scheduled.player);
        }
        view.observations.rotate_left(turn);
        for list in &mut view.observations {
            for known in list {
                known.owner = flip(known.owner);
            }
        }
        for beacon in &mut view.beacons {
            beacon.owner = flip(beacon.owner);
        }
        view.lane_hold.rotate_left(turn);
        if view.enemy_reports.len() == seats {
            view.enemy_reports.rotate_left(turn);
        }
        for (seat, _) in &mut view.eliminated {
            *seat = flip(*seat);
        }
        if let Some(initial) = &mut view.initial {
            for entity in &mut initial.entities {
                entity.owner = flip(entity.owner);
            }
            initial.players.rotate_left(turn);
            initial.gate.owner = initial.gate.owner.map(flip);
            initial.gate.capture_player = initial.gate.capture_player.map(flip);
        }
        view
    }

    pub fn visible(&self, player: u8, pos: Pos) -> bool {
        if usize::from(player) >= self.players.len() || !pos.valid(self.map.width, self.map.height)
        {
            return false;
        }
        if self.revealed {
            return true;
        }
        if pos.distance_sq(self.map.gate_pos) <= i64::from(FP * 5).pow(2) {
            return true;
        }
        // The crossing mouths are lit for both sides: a hold never starts
        // out of the other side's sight.
        let mouth_reach = i64::from(FP * CROSSING_HOLD_RADIUS_CELLS).pow(2);
        if self
            .map
            .layout()
            .mouths()
            .flatten()
            .any(|mouth| pos.distance_sq(mouth) <= mouth_reach)
        {
            return true;
        }
        // The sluice's holder sees every tidal cell.
        if self.gate.owner == Some(player) {
            let (x, y) = pos.cell_xy();
            if self.map.terrain(x, y).is_tidal() {
                return true;
            }
        }
        if self
            .beacons
            .iter()
            .any(|beacon| beacon.owner == player && beacon.until > self.tick && beacon.lights(pos))
        {
            return true;
        }
        self.entities.iter().any(|entity| {
            entity.owner == player
                && entity.hp > 0
                && entity.build_remaining == 0
                && entity.aboard.is_none()
                && entity.pos.distance_sq(pos) <= i64::from(self.sight_of(entity)).pow(2)
                && self.line_of_sight(entity.pos, pos, Some(entity.id))
        })
    }

    /// A machine's sight: a scout on Hold sees twice as far.
    pub fn sight_of(&self, entity: &Entity) -> i32 {
        let base = spec(entity.kind).sight;
        if entity.kind.is_scout() && matches!(entity.order, Order::Hold) {
            base.saturating_mul(SCOUT_HOLD_SIGHT_PERCENT) / 100
        } else {
            base
        }
    }

    /// A machine's weapon reach with the side's upgrades.
    pub fn weapon_range(&self, entity: &Entity) -> i32 {
        let base = spec(entity.kind).range;
        let player = &self.players[entity.owner as usize];
        if entity.kind == Kind::Loom && player.upgrades.contains(&Upgrade::Siege) {
            base.saturating_add(LOOM_SIEGE_RANGE_CELLS * FP)
        } else if entity.kind == Kind::Tower
            && player.doctrine_tier >= 2
            && player.doctrine == Some(Doctrine::FireControl)
        {
            base.saturating_mul(150) / 100
        } else {
            base
        }
    }

    /// Whether a wreck can be gathered by a machine of this kind now: a
    /// wreck on a flooded lane waits for the tide unless a Dredger comes.
    pub fn gatherable(&self, kind: Kind, resource: &Resource) -> bool {
        if kind == Kind::Dredger {
            return true;
        }
        let (x, y) = resource.pos.cell_xy();
        !matches!(self.depth_at(x, y), Some(Depth::Shallow | Depth::Deep))
    }

    /// The crew cap the base supports.
    pub fn crew_cap_for(&self, player: u8) -> u32 {
        let mut cap = CREW_CAP_BASE;
        for entity in &self.entities {
            if entity.owner != player || entity.hp <= 0 || entity.build_remaining > 0 {
                continue;
            }
            cap = cap.saturating_add(match entity.kind {
                Kind::Works => CREW_CAP_WORKS,
                Kind::Dropoff => CREW_CAP_YARD,
                Kind::Drydock => CREW_CAP_DRYDOCK,
                _ => 0,
            });
        }
        cap.min(CREW_CAP_MAX)
    }

    /// The pressure cap the base supports.
    pub fn pressure_cap_for(&self, player: u8) -> u32 {
        let condensers = self
            .entities
            .iter()
            .filter(|entity| {
                entity.owner == player
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.kind == Kind::Condenser
            })
            .count() as u32;
        let cap = PRESSURE_CAP_BASE
            .saturating_add(condensers.saturating_mul(PRESSURE_CAP_CONDENSER))
            .min(PRESSURE_CAP_MAX);
        if self.players[player as usize]
            .upgrades
            .contains(&Upgrade::Overpressure)
        {
            cap.saturating_add(PRESSURE_CAP_OVERPRESSURE)
        } else {
            cap
        }
    }

    fn update_caps(&mut self) {
        for player in self.seats() {
            let crew_cap = self.crew_cap_for(player);
            let pressure_cap = self.pressure_cap_for(player);
            let slot = &mut self.players[player as usize];
            slot.cap = crew_cap;
            slot.pressure_cap = pressure_cap;
            if slot.pressure > pressure_cap {
                slot.pressure = pressure_cap;
            }
        }
    }

    pub fn entity_visible(&self, player: u8, id: u32) -> bool {
        let Some(entity) = self.entity(id) else {
            return false;
        };
        entity.hp > 0
            && entity.aboard.is_none()
            && (entity.owner == player || self.visible(player, entity.pos))
    }

    pub fn knowledge(&self, player: u8) -> PlayerKnowledge {
        let mut own = Vec::new();
        let mut visible = Vec::new();
        for entity in &self.entities {
            if entity.hp <= 0 || entity.build_remaining > 0 || entity.aboard.is_some() {
                continue;
            }
            if entity.owner == player {
                own.push(KnowledgeEntity {
                    id: entity.id,
                    owner: entity.owner,
                    kind: entity.kind,
                    pos: entity.pos,
                    hp: Some(entity.hp),
                    last_seen: self.tick,
                });
            } else if self.entity_visible(player, entity.id) {
                visible.push(KnowledgeEntity {
                    id: entity.id,
                    owner: entity.owner,
                    kind: entity.kind,
                    pos: entity.pos,
                    hp: Some(entity.hp),
                    last_seen: self.tick,
                });
            }
        }
        let visible_ids: Vec<u32> = visible.iter().map(|entity| entity.id).collect();
        let stale = self.observations[player as usize]
            .iter()
            .filter(|observation| {
                !visible_ids.contains(&observation.id) && observation.kind.is_building()
            })
            .cloned()
            .map(|mut observation| {
                observation.hp = None;
                observation
            })
            .collect();
        let mut map = self.map.clone();
        for resource in &mut map.resources {
            if !self.visible(player, resource.pos) {
                resource.remaining = 0;
            }
        }
        PlayerKnowledge {
            player,
            tick: self.tick,
            own,
            visible,
            stale,
            gate: self.gate.clone(),
            map,
        }
    }

    fn spawn_starting_layout(&mut self) {
        let layout = self.map.layout();
        for player in self.seats() {
            let seat = &layout.seats[usize::from(player)];
            self.spawn_building(player, Kind::Headquarters, layout.headquarters(player), 0);
            let worker_kind = self.players[player as usize].faction.worker();
            // Every worker stands outside the headquarters' 4x4 footprint.
            for (x, y) in seat.workers {
                let id = self.spawn_unit(player, worker_kind, Pos::cell(x, y));
                if let Some(worker) = self.entity_mut(id) {
                    worker.order = Order::Idle;
                }
            }
        }
    }

    fn allocate_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.saturating_add(1);
        id
    }

    fn spawn_building(&mut self, owner: u8, kind: Kind, pos: Pos, build_remaining: u32) -> u32 {
        let id = self.allocate_entity_id();
        let health = spec(kind).health;
        self.entities.push(Entity {
            id,
            owner,
            kind,
            pos,
            hp: if build_remaining == 0 {
                health
            } else {
                (health / 4).max(1)
            },
            facing: self.map.layout().facing(owner),
            build_remaining,
            queue: Vec::new(),
            deployed: false,
            deploy_remaining: 0,
            deploy_target: false,
            after_pack: None,
            pace: 0,
            cargo: Vec::new(),
            aboard: None,
            keep_deployed: false,
            beam: None,
            build_queue: Vec::new(),
            after_build: None,
            formation: Formation::Compact,
            surge_remaining: 0,
            surge_cooldown: 0,
            order: Order::Idle,
            max_hp: health,
            carried: 0,
            carried_kind: None,
            gather_ticks: 0,
            attack_cooldown: 0,
            path: Vec::new(),
            path_index: 0,
            path_target: None,
            path_lane_revision: self.gate.lane_revision,
            blocked_ticks: 0,
            waypoints: Vec::new(),
            rally: None,
            builder: None,
            last_seen: None,
            upgrade: None,
            upgrade_queue: Vec::new(),
        });
        id
    }

    /// The work a capture of the station needs now: more for the first,
    /// while the station is still neutral (rules 15).
    pub fn capture_work(&self) -> u32 {
        if self.gate.owner.is_none() {
            CAPTURE_WORK.saturating_mul(NEUTRAL_CAPTURE_PERCENT) / 100
        } else {
            CAPTURE_WORK
        }
    }

    /// A machine's full hull with its side's upgrades: Plate on the Bulwark
    /// and Reedguard, REFIT on every combat machine, and each OVERHAUL
    /// level on every machine (rules 21).  A machine trained after an
    /// upgrade gets it too; before rules 15 only the machines in the field
    /// when Plate completed did.
    pub fn unit_max_hp(&self, owner: u8, kind: Kind) -> i32 {
        let base = spec(kind).health;
        let upgrades = &self.players[owner as usize].upgrades;
        let mut percent = 100;
        if plated(kind) && upgrades.contains(&Upgrade::Plate) {
            percent += 25;
        }
        if is_combat_unit(kind) && upgrades.contains(&Upgrade::Refit) {
            percent += REFIT_HULL_PERCENT - 100;
        }
        let mut hull = base.saturating_mul(percent) / 100;
        if !kind.is_building() {
            let levels = self.overhaul_level(owner);
            // +6% of the base hull a level, rounded to the nearest point.
            hull = hull.saturating_add(
                base.saturating_mul(OVERHAUL_HULL_PERCENT * i32::from(levels))
                    .saturating_add(50)
                    / 100,
            );
        }
        hull
    }

    /// How many OVERHAUL levels a side has completed, 0 to 3 (rules 21).
    pub fn overhaul_level(&self, owner: u8) -> u8 {
        let upgrades = &self.players[owner as usize].upgrades;
        Upgrade::OVERHAUL
            .iter()
            .filter(|level| upgrades.contains(level))
            .count() as u8
    }

    /// A combat machine's shot with its side's TEMPER.
    fn tempered(&self, owner: u8, kind: Kind, amount: i32) -> i32 {
        if !kind.is_building()
            && self.players[owner as usize]
                .upgrades
                .contains(&Upgrade::Temper)
        {
            amount.saturating_mul(TEMPER_DAMAGE_PERCENT) / 100
        } else {
            amount
        }
    }

    fn spawn_unit(&mut self, owner: u8, kind: Kind, pos: Pos) -> u32 {
        let id = self.allocate_entity_id();
        let health = self.unit_max_hp(owner, kind);
        self.entities.push(Entity {
            id,
            owner,
            kind,
            pos,
            hp: health,
            facing: self.map.layout().facing(owner),
            build_remaining: 0,
            queue: Vec::new(),
            deployed: false,
            deploy_remaining: 0,
            deploy_target: false,
            after_pack: None,
            pace: 0,
            cargo: Vec::new(),
            aboard: None,
            keep_deployed: false,
            beam: None,
            build_queue: Vec::new(),
            after_build: None,
            formation: Formation::Compact,
            surge_remaining: 0,
            surge_cooldown: 0,
            order: Order::Idle,
            max_hp: health,
            carried: 0,
            carried_kind: None,
            gather_ticks: 0,
            attack_cooldown: 0,
            path: Vec::new(),
            path_index: 0,
            path_target: None,
            path_lane_revision: self.gate.lane_revision,
            blocked_ticks: 0,
            waypoints: Vec::new(),
            rally: None,
            builder: None,
            last_seen: None,
            upgrade: None,
            upgrade_queue: Vec::new(),
        });
        id
    }

    /// The starting workers go to the wreck bed nearest their headquarters
    /// (the first on a tie: on the Split Basin, the northern home bed).
    fn assign_idle_workers(&mut self, player: u8) {
        let home = self.map.layout().headquarters(player);
        let resource = self
            .map
            .resources
            .iter()
            .filter(|resource| resource.kind == ResourceKind::Salvage && resource.remaining > 0)
            .min_by_key(|resource| (resource.pos.distance_sq(home), resource.id))
            .map(|resource| resource.id);
        let Some(resource) = resource else {
            return;
        };
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.owner == player && entity.kind.is_worker())
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            if let Some(entity) = self.entity_mut(id) {
                entity.order = Order::Gather { resource };
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
            }
        }
    }

    /// Test support for the other crates: a finished building or a machine
    /// placed directly, outside any rule.
    #[doc(hidden)]
    pub fn spawn_for_tests(&mut self, owner: u8, kind: Kind, pos: Pos) -> u32 {
        if kind.is_building() {
            self.spawn_building(owner, kind, pos, 0)
        } else {
            self.spawn_unit(owner, kind, pos)
        }
    }

    fn entity(&self, id: u32) -> Option<&Entity> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    fn entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|entity| entity.id == id)
    }

    fn clear_order(&mut self, id: u32) {
        if let Some(entity) = self.entity_mut(id) {
            entity.pace = 0;
            entity.order = Order::Idle;
            entity.path.clear();
            entity.path_index = 0;
            entity.path_target = None;
            entity.waypoints.clear();
            entity.gather_ticks = 0;
            entity.after_pack = None;
        }
    }

    fn initial_state(&self) -> InitialState {
        InitialState {
            seed: self.seed,
            map: self.map.clone(),
            gate: self.gate.clone(),
            entities: self.entities.clone(),
            players: self.players.clone(),
            next_entity: self.next_entity_id,
            rng_state: self.rng_state,
        }
    }

    fn from_initial(initial: InitialState) -> Result<Self, String> {
        let world = Self {
            tick: 0,
            entities: initial.entities.clone(),
            players: initial.players.clone(),
            map: initial.map.clone(),
            gate: initial.gate.clone(),
            artillery: Vec::new(),
            outcome: None,
            events: Vec::new(),
            command_log: Vec::new(),
            pending: Vec::new(),
            initial: Some(initial.clone()),
            seed: initial.seed,
            rng_state: initial.rng_state,
            lane_revision: initial.gate.lane_revision,
            ai_enabled: false,
            ai_last_tick: 0,
            ai_level: AiLevel::Normal,
            replaying_ai: false,
            next_entity_id: initial.next_entity,
            observations: vec![Vec::new(); initial.players.len()],
            beacons: Vec::new(),
            crust: Vec::new(),
            lane_hold: vec![0; initial.players.len()],
            enemy_reports: vec![Vec::new(); initial.players.len()],
            eliminated: Vec::new(),
            revealed: false,
            view_turn: 0,
        };
        world.validate_invariants()?;
        Ok(world)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        self.validate_invariants()?;
        let payload =
            serde_json::to_vec(self).map_err(|error| format!("serialize save payload: {error}"))?;
        let envelope = SaveEnvelope {
            version: SIM_VERSION,
            rules_digest: bw_content::rules_digest(),
            state_hash: self.state_hash(),
            payload_digest: blake3::hash(&payload).to_hex().to_string(),
            world: self.clone(),
        };
        let bytes =
            serde_json::to_vec(&envelope).map_err(|error| format!("serialize save: {error}"))?;
        write_bounded(path.as_ref(), &bytes)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = read_bounded(path.as_ref())?;
        let raw: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| format!("parse save: {error}"))?;
        let version = raw
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "save is missing its schema version".to_string())?;
        if version != u64::from(SIM_VERSION) {
            return Err(format!("unsupported save version {version}"));
        }
        let envelope: SaveEnvelope =
            serde_json::from_value(raw).map_err(|error| format!("parse save envelope: {error}"))?;
        if envelope.rules_digest != bw_content::rules_digest() {
            return Err("save rules digest mismatch".to_string());
        }
        let payload = serde_json::to_vec(&envelope.world)
            .map_err(|error| format!("serialize loaded payload: {error}"))?;
        if envelope.payload_digest != blake3::hash(&payload).to_hex().to_string() {
            return Err("save payload checksum mismatch".to_string());
        }
        if envelope.state_hash != envelope.world.state_hash() {
            return Err("save state hash mismatch".to_string());
        }
        let world = envelope.world;
        if world.initial.is_none() {
            return Err("save is missing its initial replay state".to_string());
        }
        world.validate_invariants()?;
        Ok(world)
    }

    pub fn state_hash(&self) -> String {
        let mut pending = self.pending.clone();
        pending.sort_by_key(|item| (item.execute_tick, item.player, item.sequence));
        let mut command_log = self.command_log.clone();
        command_log.sort_by_key(|record| (record.tick, record.player, record.sequence));
        let canonical = CanonicalState {
            tick: self.tick,
            entities: self.entities.clone(),
            players: self.players.clone(),
            map: self.map.clone(),
            gate: self.gate.clone(),
            artillery: self.artillery.clone(),
            outcome: self.outcome.clone(),
            command_log,
            pending,
            seed: self.seed,
            rng_state: self.rng_state,
            lane_revision: self.lane_revision,
            ai_enabled: self.ai_enabled,
            ai_last_tick: self.ai_last_tick,
            ai_level: self.ai_level,
            next_entity_id: self.next_entity_id,
            observations: self.observations.clone(),
            lane_hold: self.lane_hold.clone(),
            eliminated: self.eliminated.clone(),
            crust: self.crust.clone(),
        };
        let bytes = serde_json::to_vec(&canonical)
            .expect("canonical state contains only bounded serde values");
        blake3::hash(&bytes).to_hex().to_string()
    }

    pub fn export_replay(&self, path: impl AsRef<Path>) -> Result<(), String> {
        self.validate_invariants()?;
        let initial = self
            .initial
            .clone()
            .ok_or_else(|| "world has no initial state".to_string())?;
        let replay = ReplayFile {
            version: SIM_VERSION,
            rules_digest: bw_content::rules_digest(),
            initial,
            commands: self.command_log.clone(),
            end_tick: self.tick,
            ai_enabled: self.ai_enabled,
            ai_last_tick: self.ai_last_tick,
            ai_level: self.ai_level,
        };
        let bytes =
            serde_json::to_vec(&replay).map_err(|error| format!("serialize replay: {error}"))?;
        write_bounded(path.as_ref(), &bytes)
    }

    /// Read a replay file's envelope, checked for version, digest and bounds.
    fn read_replay_file(path: &Path) -> Result<ReplayFile, String> {
        let bytes = read_bounded(path)?;
        let raw: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| format!("parse replay: {error}"))?;
        let version = raw
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "replay is missing its schema version".to_string())?;
        if version != u64::from(SIM_VERSION) {
            return Err(format!("unsupported replay version {version}"));
        }
        let replay: ReplayFile = serde_json::from_value(raw)
            .map_err(|error| format!("parse replay envelope: {error}"))?;
        if replay.rules_digest != bw_content::rules_digest() {
            return Err("replay rules digest mismatch".to_string());
        }
        if replay.commands.len() > MAX_COMMANDS || replay.end_tick > MAX_REPLAY_TICKS {
            return Err("replay bounds exceeded".to_string());
        }
        Ok(replay)
    }

    pub fn replay(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = read_bounded(path.as_ref())?;
        let raw: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| format!("parse replay: {error}"))?;
        let version = raw
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "replay is missing its schema version".to_string())?;
        if version != u64::from(SIM_VERSION) {
            return Err(format!("unsupported replay version {version}"));
        }
        let replay: ReplayFile = serde_json::from_value(raw)
            .map_err(|error| format!("parse replay envelope: {error}"))?;
        if replay.rules_digest != bw_content::rules_digest() {
            return Err("replay rules digest mismatch".to_string());
        }
        if replay.commands.len() > MAX_COMMANDS || replay.end_tick > MAX_REPLAY_TICKS {
            return Err("replay bounds exceeded".to_string());
        }
        let mut world = Self::from_initial(replay.initial)?;
        world.ai_level = replay.ai_level;
        let replay_ai_enabled = replay.ai_enabled;
        let replay_ai_last_tick = replay.ai_last_tick;
        world.ai_enabled = false;
        world.replaying_ai = replay_ai_enabled;
        let mut records = replay.commands;
        records.sort_by_key(|record| (record.tick, record.player, record.sequence));
        let mut index = 0usize;
        while world.tick < replay.end_tick {
            while index < records.len() && records[index].tick == world.tick.saturating_add(1) {
                let record = records[index].clone();
                let result = world.issue(record.player, record.command.clone());
                if result.is_ok() != record.accepted {
                    return Err(format!(
                        "replay command outcome differs at tick {}",
                        record.tick
                    ));
                }
                index += 1;
            }
            // A finished world no longer advances its tick: a replay that
            // ends before its recording did has parted from the match.
            if world.outcome.is_some() {
                return Err(format!(
                    "replay ended at tick {} before its recorded end {}",
                    world.tick, replay.end_tick
                ));
            }
            world.step();
        }
        if index != records.len() {
            return Err("replay contains a command outside its end tick".to_string());
        }
        world.ai_enabled = replay_ai_enabled;
        world.replaying_ai = false;
        world.ai_last_tick = replay_ai_last_tick;
        world.validate_invariants()?;
        Ok(world)
    }

    fn validate_invariants(&self) -> Result<(), String> {
        validate_map_bounds(&self.map)?;
        let expected_tiles = usize::from(self.map.width)
            .checked_mul(usize::from(self.map.height))
            .ok_or_else(|| "map dimensions overflow".to_string())?;
        if self.map.width == 0
            || self.map.height == 0
            || self.map.width > 512
            || self.map.height > 512
            || self.map.tiles.len() != expected_tiles
            || !self.map.gate_pos.valid(self.map.width, self.map.height)
        {
            return Err("map dimensions or tile count invalid".to_string());
        }
        if self.entities.len() > MAX_ENTITIES
            || self.command_log.len() > MAX_COMMANDS
            || self.pending.len() > MAX_COMMANDS
            || self.map.resources.len() > MAX_ENTITIES
            || self.map.wells.len() > MAX_ENTITIES
            || self.artillery.len() > MAX_ARTILLERY_SHOTS
            || self
                .players
                .iter()
                .any(|side| side.worker_danger.len() > MAX_DANGER_MARKS)
        {
            return Err("world bounds exceeded".to_string());
        }
        let layout = self.map.layout();
        if self.players.len() < 2
            || self.players.len() > MAX_SEATS
            || self.players.len() != layout.seat_count()
            || usize::from(self.map.width) != usize::from(layout.size)
            || self.gate.dry_arm.index() >= layout.arm_count()
            || self
                .gate
                .switch_target
                .is_some_and(|arm| arm.index() >= layout.arm_count())
            || self.lane_hold.len() != self.players.len()
            || self.observations.len() != self.players.len()
            || self
                .eliminated
                .iter()
                .any(|&(seat, _)| usize::from(seat) >= self.players.len())
        {
            return Err("seat count or per-seat state invalid".to_string());
        }
        if self.entities.iter().any(|entity| {
            entity.upgrade_queue.len() > UPGRADE_QUEUE
                || (entity.upgrade.is_none() && !entity.upgrade_queue.is_empty())
        }) {
            return Err("upgrade queue invalid".to_string());
        }
        let mut last_id = 0u32;
        for entity in &self.entities {
            if entity.id == 0
                || entity.id <= last_id
                || usize::from(entity.owner) >= self.players.len()
                || entity.hp <= 0
                || entity.hp > entity.max_hp
                || entity.facing > 7
                || entity.deploy_remaining > TICK_HZ as u32
                || entity.surge_remaining > SURGE_TICKS
                || entity.surge_cooldown > SURGE_COOLDOWN_TICKS
                || !entity.pos.valid(self.map.width, self.map.height)
                || entity.queue.len() > MAX_QUEUE
                || entity.path.len() > MAX_PATH
                || entity.path_index > entity.path.len()
                || entity
                    .path_target
                    .is_some_and(|position| !position.valid(self.map.width, self.map.height))
                || entity
                    .rally
                    .is_some_and(|position| !position.valid(self.map.width, self.map.height))
                || entity.waypoints.len() > MAX_WAYPOINTS
            {
                return Err(format!("entity {} invariant failed", entity.id));
            }
            if entity
                .path
                .iter()
                .chain(entity.waypoints.iter())
                .any(|position| !position.valid(self.map.width, self.map.height))
            {
                return Err(format!(
                    "entity {} navigation coordinate invalid",
                    entity.id
                ));
            }
            if !self.order_valid(&entity.order) {
                return Err(format!("entity {} order coordinate invalid", entity.id));
            }
            last_id = entity.id;
        }
        if self.next_entity_id == 0 || self.next_entity_id <= last_id {
            return Err("next entity ID is not monotonic".to_string());
        }
        for player in &self.players {
            if player.pressure > PRESSURE_CAP_MAX + PRESSURE_CAP_OVERPRESSURE
                || player.crew > player.cap
                || player.cap > 1000
            {
                return Err("player invariant failed".to_string());
            }
            if player.doctrine.is_some() && player.research.is_some() {
                return Err("player has completed and active doctrines".to_string());
            }
            if let Some(research) = player.research
                && (research.remaining == 0 || research.remaining > DOCTRINE_TIER2_TICKS)
            {
                return Err("research timer exceeds doctrine duration".to_string());
            }
        }
        for (player_index, player) in self.players.iter().enumerate() {
            if let Some(research) = player.research
                && !self.entities.iter().any(|entity| {
                    entity.id == research.building
                        && entity.owner == player_index as u8
                        && entity.kind == Kind::Headquarters
                        && entity.hp > 0
                        && entity.build_remaining == 0
                })
            {
                return Err("research building is missing or unfinished".to_string());
            }
        }
        for shot in &self.artillery {
            if usize::from(shot.owner) >= self.players.len()
                || shot.source == 0
                || !shot.from.valid(self.map.width, self.map.height)
                || !shot.target.valid(self.map.width, self.map.height)
                || shot.source >= self.next_entity_id
                || shot.impact_tick < self.tick
                || shot.impact_tick > self.tick.saturating_add(u64::from(LOOM_WINDUP_TICKS))
            {
                return Err("artillery shot invariant failed".to_string());
            }
        }
        let mut last_resource = 0u32;
        for resource in &self.map.resources {
            if resource.id == 0
                || resource.id <= last_resource
                || !resource.pos.valid(self.map.width, self.map.height)
                || resource.kind != ResourceKind::Salvage
            {
                return Err("resource invariant failed".to_string());
            }
            last_resource = resource.id;
        }
        for pending in &self.pending {
            if pending.log_index >= self.command_log.len() {
                return Err("pending command log index invalid".to_string());
            }
            if usize::from(pending.player) >= self.players.len()
                || pending.execute_tick < self.tick
                || pending.execute_tick > self.tick.saturating_add(MAX_REPLAY_TICKS)
            {
                return Err("pending command timing invalid".to_string());
            }
            self.validate_command_bounds(&pending.command)?;
        }
        for record in &self.command_log {
            if usize::from(record.player) >= self.players.len()
                || record.command.command_target_len() > MAX_SELECTION
                || record
                    .reason
                    .as_ref()
                    .is_some_and(|reason| reason.len() > 512)
            {
                return Err("command record bounds invalid".to_string());
            }
            self.validate_command_bounds(&record.command)?;
        }
        if let Some(initial) = &self.initial {
            validate_map_bounds(&initial.map)?;
            if initial.entities.len() > MAX_ENTITIES
                || initial.players.iter().any(|player| player.cap > 1000)
                || initial.next_entity == 0
            {
                return Err("initial state bounds invalid".to_string());
            }
            let initial_max_id = initial
                .entities
                .iter()
                .map(|entity| entity.id)
                .max()
                .unwrap_or(0);
            if initial.next_entity <= initial_max_id {
                return Err("initial entity ID is not monotonic".to_string());
            }
            for entity in &initial.entities {
                if !entity.pos.valid(initial.map.width, initial.map.height)
                    || entity.path.len() > MAX_PATH
                    || entity.waypoints.len() > MAX_WAYPOINTS
                    || entity.path_index > entity.path.len()
                    || entity.path_target.is_some_and(|position| {
                        !position.valid(initial.map.width, initial.map.height)
                    })
                    || entity.rally.is_some_and(|position| {
                        !position.valid(initial.map.width, initial.map.height)
                    })
                    || entity
                        .path
                        .iter()
                        .chain(entity.waypoints.iter())
                        .any(|position| !position.valid(initial.map.width, initial.map.height))
                {
                    return Err("initial navigation bounds invalid".to_string());
                }
            }
        }
        for observations in &self.observations {
            if observations.len() > MAX_ENTITIES
                || observations
                    .windows(2)
                    .any(|window| window[0].id >= window[1].id)
            {
                return Err("fog observation bounds invalid".to_string());
            }
            if observations.iter().any(|observation| {
                observation.id == 0
                    || usize::from(observation.owner) >= self.players.len()
                    || !observation.pos.valid(self.map.width, self.map.height)
            }) {
                return Err("fog observation coordinate invalid".to_string());
            }
        }
        if self.gate.warning_until.is_none()
            && (self.gate.switch_target.is_some()
                || self.gate.flood_pending
                || self.gate.ebb_pending)
        {
            return Err("gate target without warning".to_string());
        }
        if self.gate.ebb_pending && (self.gate.switch_target.is_some() || self.gate.flood_pending) {
            return Err("an ebb shares its warning".to_string());
        }
        if self.gate.capture_progress > self.capture_work() {
            return Err("gate capture progress invalid".to_string());
        }
        Ok(())
    }

    fn order_valid(&self, order: &Order) -> bool {
        match order {
            Order::Move { target } | Order::AttackMove { target } => {
                target.valid(self.map.width, self.map.height)
            }
            _ => true,
        }
    }

    fn validate_command_bounds(&self, command: &Command) -> Result<(), String> {
        let check_units = |units: &[u32]| {
            if units.is_empty() || units.len() > MAX_SELECTION {
                Err("command selection bound invalid".to_string())
            } else {
                Ok(())
            }
        };
        match command {
            Command::Move { units, target, .. } | Command::AttackMove { units, target, .. } => {
                check_units(units)?;
                if !target.valid(self.map.width, self.map.height) {
                    return Err("command target coordinate invalid".to_string());
                }
            }
            Command::Attack { units, .. }
            | Command::Gather { units, .. }
            | Command::QueueGather { units, .. }
            | Command::Stop { units }
            | Command::Hold { units }
            | Command::Deploy { units }
            | Command::Capture { units }
            | Command::SetDeployed { units, .. }
            | Command::KeepDeployed { units, .. }
            | Command::Repair { units, .. }
            | Command::SetFormation { units, .. }
            | Command::Face { units, .. }
            | Command::Surge { units }
            | Command::Deliver { units, .. }
            | Command::Recycle { units } => check_units(units)?,
            Command::Build { pos, .. }
            | Command::Rally { pos, .. }
            | Command::Glint { target: pos, .. }
            | Command::Lay { target: pos, .. } => {
                if !pos.valid(self.map.width, self.map.height) {
                    return Err("command coordinate invalid".to_string());
                }
            }
            Command::Train { .. }
            | Command::Cancel { .. }
            | Command::Research { .. }
            | Command::CancelResearch { .. }
            | Command::Upgrade { .. }
            | Command::CancelUpgrade { .. }
            | Command::Vent { .. }
            | Command::Reclaim { .. }
            | Command::Sound { .. }
            | Command::SwitchGate
            | Command::SetTide { .. }
            | Command::Flood
            | Command::Board { .. }
            | Command::Unload { .. }
            | Command::Surrender => {}
        }
        Ok(())
    }
}

/// A replay played back one tick at a time, for an observer that watches a
/// finished or still-growing match with the fog lifted.
pub struct ReplayPlayer {
    world: World,
    records: Vec<CommandRecord>,
    index: usize,
    end_tick: u64,
}

impl ReplayPlayer {
    /// Open a replay file at its first tick.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let replay = World::read_replay_file(path.as_ref())?;
        let mut world = World::from_initial(replay.initial)?;
        world.ai_level = replay.ai_level;
        world.replaying_ai = replay.ai_enabled;
        let mut records = replay.commands;
        records.sort_by_key(|record| (record.tick, record.player, record.sequence));
        Ok(Self {
            world,
            records,
            index: 0,
            end_tick: replay.end_tick,
        })
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn tick(&self) -> u64 {
        self.world.tick
    }

    pub fn end_tick(&self) -> u64 {
        self.end_tick
    }

    pub fn finished(&self) -> bool {
        self.world.tick >= self.end_tick || self.world.outcome.is_some()
    }

    /// Apply the next tick's commands and step once.  Returns false at the
    /// end of the recording.
    pub fn step(&mut self) -> Result<bool, String> {
        if self.finished() {
            return Ok(false);
        }
        while self.index < self.records.len()
            && self.records[self.index].tick == self.world.tick.saturating_add(1)
        {
            let record = self.records[self.index].clone();
            let result = self.world.issue(record.player, record.command.clone());
            if result.is_ok() != record.accepted {
                return Err(format!(
                    "replay command outcome differs at tick {}",
                    record.tick
                ));
            }
            self.index += 1;
        }
        self.world.step();
        Ok(true)
    }

    /// Continue from a world this recording passed through, such as a
    /// snapshot taken while playing it: the seek behind a replay timeline.
    /// Commands at or before the world's tick count as applied.
    pub fn restore(&mut self, world: World) {
        self.index = self
            .records
            .partition_point(|record| record.tick <= world.tick);
        let replaying_ai = self.world.replaying_ai;
        self.world = world;
        self.world.replaying_ai = replaying_ai;
    }

    /// Re-read a still-growing replay file and take up any commands and end
    /// tick it has gained.  The file must be the same match: its records must
    /// start with the ones already played.
    pub fn extend_from(&mut self, path: impl AsRef<Path>) -> Result<bool, String> {
        let replay = World::read_replay_file(path.as_ref())?;
        if replay.initial.seed != self.world.seed {
            return Err("replay file is a different match".to_string());
        }
        let mut records = replay.commands;
        records.sort_by_key(|record| (record.tick, record.player, record.sequence));
        if records.len() < self.records.len() || records[..self.records.len()] != self.records[..] {
            return Err("replay file no longer matches the played commands".to_string());
        }
        let grew = records.len() > self.records.len() || replay.end_tick > self.end_tick;
        self.records = records;
        self.end_tick = self.end_tick.max(replay.end_tick);
        Ok(grew)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CanonicalState {
    tick: u64,
    entities: Vec<Entity>,
    players: Vec<Player>,
    map: Map,
    gate: Gate,
    artillery: Vec<ArtilleryShot>,
    outcome: Option<Outcome>,
    command_log: Vec<CommandRecord>,
    pending: Vec<ScheduledCommand>,
    seed: u64,
    rng_state: u64,
    lane_revision: u32,
    ai_enabled: bool,
    ai_last_tick: u64,
    #[serde(skip_serializing_if = "AiLevel::is_normal")]
    ai_level: AiLevel,
    next_entity_id: u32,
    observations: Vec<Vec<KnowledgeEntity>>,
    lane_hold: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    eliminated: Vec<(u8, u64)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    crust: Vec<u32>,
}

fn write_bounded(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_SAVE_BYTES {
        return Err("serialized file exceeds limit".to_string());
    }
    let temporary = temp_path(path);
    fs::write(&temporary, bytes).map_err(|error| format!("write temporary file: {error}"))?;
    if path.exists() {
        let backup = backup_path(path);
        let _ = fs::copy(path, backup);
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("replace save: {error}"));
    }
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("read file metadata: {error}"))?;
    if metadata.len() > MAX_SAVE_BYTES as u64 {
        return Err("serialized file exceeds limit".to_string());
    }
    fs::read(path).map_err(|error| format!("read file: {error}"))
}

fn temp_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".tmp");
    PathBuf::from(value)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".bak");
    PathBuf::from(value)
}

fn validate_map_bounds(map: &Map) -> Result<(), String> {
    let expected = usize::from(map.width)
        .checked_mul(usize::from(map.height))
        .ok_or_else(|| "map dimensions overflow".to_string())?;
    if map.width == 0
        || map.height == 0
        || map.width > 512
        || map.height > 512
        || map.tiles.len() != expected
        || !map.gate_pos.valid(map.width, map.height)
        || map.wells.len() > MAX_ENTITIES
    {
        return Err("map dimensions or tile count invalid".to_string());
    }
    let mut resource_id = 0u32;
    for resource in &map.resources {
        if resource.id == 0
            || resource.id <= resource_id
            || resource.kind != ResourceKind::Salvage
            || !resource.pos.valid(map.width, map.height)
        {
            return Err("map resource bounds invalid".to_string());
        }
        resource_id = resource.id;
    }
    if map
        .wells
        .iter()
        .any(|well| !well.valid(map.width, map.height))
    {
        return Err("map well coordinate invalid".to_string());
    }
    Ok(())
}

impl World {
    fn validate_submission(&self, player: u8, command: &Command) -> Result<(), String> {
        match command {
            Command::Move { units, target, .. } | Command::AttackMove { units, target, .. } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    self.entity(*id).is_some_and(|entity| {
                        entity.waypoints.len().saturating_add(1) > MAX_WAYPOINTS
                    })
                }) {
                    return Err("waypoint queue is full".to_string());
                }
                self.validate_target(*target)
            }
            Command::Attack { units, target } => {
                self.validate_units(player, units)?;
                if !self.entity_visible(player, *target) {
                    return Err("target is not visible".to_string());
                }
                let target_entity = self
                    .entity(*target)
                    .ok_or_else(|| "target missing".to_string())?;
                if target_entity.owner == player {
                    return Err("cannot attack allied entity".to_string());
                }
                Ok(())
            }
            Command::Gather { units, resource } | Command::QueueGather { units, resource } => {
                self.validate_units(player, units)?;
                if units
                    .iter()
                    .any(|id| !self.entity(*id).is_some_and(|entity| entity.kind.gathers()))
                {
                    return Err("only workers and Dredgers gather".to_string());
                }
                if self
                    .map
                    .resources
                    .iter()
                    .all(|item| item.id != *resource || item.remaining == 0)
                {
                    return Err("resource missing or exhausted".to_string());
                }
                if self
                    .map
                    .resources
                    .iter()
                    .find(|item| item.id == *resource)
                    .is_some_and(|item| item.kind != ResourceKind::Salvage)
                {
                    return Err("pressure comes from condensers, not workers".to_string());
                }
                Ok(())
            }
            Command::Build {
                worker, kind, pos, ..
            } => {
                let entity = self
                    .entity(*worker)
                    .ok_or_else(|| "worker missing".to_string())?;
                if entity.owner != player || !entity.kind.is_worker() {
                    return Err("builder is not your worker".to_string());
                }
                if !faction_allows(self.players[player as usize].faction, *kind)
                    || *kind == Kind::Headquarters
                {
                    return Err("building kind is unavailable".to_string());
                }
                self.validate_target(*pos)?;
                if !self.can_place_for(Some(player), *kind, *pos) {
                    return Err("building placement is blocked".to_string());
                }
                let content = spec(*kind);
                if self.players[player as usize].salvage < content.salvage
                    || self.players[player as usize].pressure < content.pressure
                {
                    return Err("insufficient resources".to_string());
                }
                Ok(())
            }
            Command::Train { building, kind } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "production building missing".to_string())?;
                if entity.owner != player || entity.build_remaining > 0 || !entity.kind.produces() {
                    return Err("invalid production building".to_string());
                }
                if entity.queue.len() >= MAX_QUEUE {
                    return Err("production queue is full".to_string());
                }
                if !trains(self.players[player as usize].faction, entity.kind, *kind) {
                    return Err("unit kind is unavailable".to_string());
                }
                let content = spec(*kind);
                let queued_crew: u32 = entity.queue.iter().map(|item| spec(item.kind).crew).sum();
                if self.players[player as usize]
                    .crew
                    .saturating_add(queued_crew)
                    .saturating_add(content.crew)
                    > self.players[player as usize].cap
                {
                    return Err("crew capacity exceeded".to_string());
                }
                if self.players[player as usize].salvage < content.salvage
                    || self.players[player as usize].pressure < content.pressure
                {
                    return Err("insufficient resources".to_string());
                }
                if matches!(content.movement, Movement::Hull)
                    && self.find_spawn_pos_for(*kind, entity.pos).is_none()
                {
                    return Err("no water within reach of this drydock".to_string());
                }
                Ok(())
            }
            Command::Cancel { building } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "building missing".to_string())?;
                if entity.owner != player
                    || (entity.build_remaining == 0 && entity.queue.is_empty())
                {
                    return Err("nothing to cancel".to_string());
                }
                Ok(())
            }
            Command::Rally { building, pos } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "building missing".to_string())?;
                // Every building that trains takes a rally, the Drydock
                // included (rules 14).
                if entity.owner != player || !entity.kind.produces() {
                    return Err("invalid rally building".to_string());
                }
                self.validate_target(*pos)
            }
            Command::Stop { units }
            | Command::Hold { units }
            | Command::Deploy { units }
            | Command::Capture { units } => {
                self.validate_units(player, units)?;
                // A surging machine may take a capture order: it runs on and
                // starts channelling when the surge ends (rules 14).
                if matches!(command, Command::Deploy { .. })
                    && units.iter().any(|id| {
                        self.entity(*id)
                            .is_some_and(|entity| entity.surge_remaining > 0)
                    })
                {
                    return Err("Surge is active; wait for it to end".to_string());
                }
                if matches!(command, Command::Capture { .. })
                    && units.iter().any(|id| {
                        self.entity(*id).is_some_and(|entity| {
                            entity.kind.is_worker() || entity.kind.is_building()
                        })
                    })
                {
                    return Err("workers and buildings cannot capture".to_string());
                }
                if matches!(command, Command::Deploy { .. })
                    && units.iter().any(|id| {
                        !self.entity(*id).is_some_and(|entity| {
                            is_specialist(entity.kind) && entity.deploy_remaining == 0
                        })
                    })
                {
                    return Err("unit has no deploy action".to_string());
                }
                Ok(())
            }
            Command::SetDeployed { units, deployed } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    !self
                        .entity(*id)
                        .is_some_and(|entity| is_specialist(entity.kind))
                }) {
                    return Err("unit has no deploy action".to_string());
                }
                if *deployed
                    && units.iter().any(|id| {
                        self.entity(*id)
                            .is_some_and(|entity| entity.surge_remaining > 0)
                    })
                {
                    return Err("Surge is active; wait for it to end".to_string());
                }
                Ok(())
            }
            Command::KeepDeployed { units, .. } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    !self
                        .entity(*id)
                        .is_some_and(|entity| is_specialist(entity.kind))
                }) {
                    return Err("unit has no deploy action".to_string());
                }
                Ok(())
            }
            Command::SwitchGate | Command::SetTide { .. } | Command::Flood => {
                if self.gate.owner != Some(player) {
                    return Err("player does not control the gate".to_string());
                }
                if self.tick < self.gate.locked_until || self.gate.warning_until.is_some() {
                    return Err("gate is locked or already warning".to_string());
                }
                if self.gate.tide == Tide::Flood {
                    return Err("the flood must fall first".to_string());
                }
                let price = if matches!(command, Command::Flood) {
                    FLOOD_PRESSURE
                } else {
                    SWITCH_PRESSURE
                };
                if self.players[player as usize].pressure < price {
                    return Err("insufficient pressure for the tide".to_string());
                }
                if self.station_contested_for(player) {
                    return Err("an enemy stands at the station".to_string());
                }
                if let Command::SetTide { arm } = command {
                    if arm.index() >= self.map.layout().arm_count() {
                        return Err("the map has no such arm".to_string());
                    }
                    if self.gate.tide == Tide::Open && self.gate.dry_arm == *arm {
                        return Err("that side is open already".to_string());
                    }
                }
                Ok(())
            }
            Command::Board { units, transport } => {
                self.validate_units(player, units)?;
                let carrier = self
                    .entity(*transport)
                    .ok_or_else(|| "transport missing".to_string())?;
                if carrier.owner != player || carrier.hp <= 0 || !carrier.kind.is_transport() {
                    return Err("board an own transport".to_string());
                }
                if units.iter().any(|id| {
                    self.entity(*id).is_none_or(|entity| {
                        entity.kind.is_building()
                            || entity.kind.is_transport()
                            || entity.aboard.is_some()
                            || entity.deployed
                            || entity.deploy_remaining > 0
                    })
                }) {
                    return Err("only packed machines on the ground can board".to_string());
                }
                Ok(())
            }
            Command::Unload { transport } => {
                let carrier = self
                    .entity(*transport)
                    .ok_or_else(|| "transport missing".to_string())?;
                if carrier.owner != player || carrier.hp <= 0 || !carrier.kind.is_transport() {
                    return Err("unload an own transport".to_string());
                }
                if carrier.cargo.is_empty() {
                    return Err("the hold is empty".to_string());
                }
                Ok(())
            }
            Command::Repair { units, target } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    !self
                        .entity(*id)
                        .is_some_and(|entity| entity.kind.is_worker())
                }) {
                    return Err("only workers repair".to_string());
                }
                let target_entity = self
                    .entity(*target)
                    .ok_or_else(|| "repair target missing".to_string())?;
                if target_entity.owner != player || !target_entity.kind.is_building() {
                    return Err("repair target is not allied".to_string());
                }
                Ok(())
            }
            Command::Deliver { units, target } => {
                self.validate_units(player, units)?;
                if units
                    .iter()
                    .any(|id| !self.entity(*id).is_some_and(|entity| entity.kind.gathers()))
                {
                    return Err("only workers and Dredgers deliver".to_string());
                }
                if !self
                    .entity(*target)
                    .is_some_and(|building| delivers_at(building, player))
                {
                    return Err("deliver at your headquarters or Salvage Yard".to_string());
                }
                Ok(())
            }
            Command::Recycle { units } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    !self
                        .entity(*id)
                        .is_some_and(|entity| entity.kind.is_worker())
                }) {
                    return Err("only workers are recycled".to_string());
                }
                if !self
                    .entities
                    .iter()
                    .any(|entity| recycles_at(entity, player))
                {
                    return Err(
                        "RECYCLE needs your finished headquarters, Works or Salvage Yard"
                            .to_string(),
                    );
                }
                Ok(())
            }
            Command::SetFormation { units, .. } => {
                self.validate_units(player, units)?;
                if units.iter().any(|id| {
                    self.entity(*id)
                        .is_some_and(|entity| entity.kind.is_building())
                }) {
                    return Err("buildings cannot use army formations".to_string());
                }
                Ok(())
            }
            Command::Face { units, target } => {
                self.validate_units(player, units)?;
                self.validate_target(*target)?;
                if units.iter().any(|id| {
                    self.entity(*id)
                        .is_some_and(|entity| entity.kind.is_worker() || entity.kind.is_building())
                }) {
                    return Err("only mobile combat units can face".to_string());
                }
                if units.iter().any(|id| {
                    self.entity(*id).is_some_and(|entity| {
                        is_specialist(entity.kind)
                            && (entity.deployed || entity.deploy_remaining > 0)
                    })
                }) {
                    return Err("pack deployed specialist before facing".to_string());
                }
                Ok(())
            }
            Command::Surge { units } => {
                self.validate_units(player, units)?;
                // The able machines of a mixed group surge and the rest are
                // skipped (rules 14); only a group with none able is refused.
                let able = self.surge_able(player, units);
                if able.is_empty() {
                    return Err(
                        "no machine here can surge: it needs a packed combat machine off cooldown"
                            .to_string(),
                    );
                }
                let cost = SURGE_PRESSURE.saturating_mul(able.len() as u32);
                if self.players[player as usize].pressure < cost {
                    return Err("insufficient pressure for selected Surge units".to_string());
                }
                Ok(())
            }
            Command::Research { building, doctrine } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "research headquarters missing".to_string())?;
                if entity.owner != player
                    || entity.kind != Kind::Headquarters
                    || entity.build_remaining > 0
                {
                    return Err("research requires a finished headquarters".to_string());
                }
                let (salvage, pressure) = self.research_price(player, *doctrine)?;
                if self.players[player as usize].salvage < salvage
                    || self.players[player as usize].pressure < pressure
                {
                    return Err("insufficient resources for doctrine research".to_string());
                }
                Ok(())
            }
            Command::Upgrade { building, upgrade } => {
                self.upgrade_allowed(player, *building, *upgrade)?;
                let (salvage, pressure, _) = upgrade.cost();
                if self.players[player as usize].salvage < salvage
                    || self.players[player as usize].pressure < pressure
                {
                    return Err("insufficient resources for the upgrade".to_string());
                }
                Ok(())
            }
            Command::CancelUpgrade { building } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "building missing".to_string())?;
                if entity.owner != player || entity.upgrade.is_none() {
                    return Err("no upgrade to cancel".to_string());
                }
                Ok(())
            }
            Command::Vent { building } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "headquarters missing".to_string())?;
                if entity.owner != player
                    || entity.kind != Kind::Headquarters
                    || entity.build_remaining > 0
                {
                    return Err("VENT needs your finished headquarters".to_string());
                }
                if self.players[player as usize].vent_remaining > 0 {
                    return Err("VENT is already open".to_string());
                }
                if self.players[player as usize].pressure < VENT_PRESSURE {
                    return Err("insufficient pressure for VENT".to_string());
                }
                Ok(())
            }
            Command::Reclaim { building } => {
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "headquarters missing".to_string())?;
                if entity.owner != player
                    || entity.kind != Kind::Headquarters
                    || entity.build_remaining > 0
                {
                    return Err("RECLAIM needs your finished headquarters".to_string());
                }
                if self.players[player as usize].pressure < RECLAIM_PRESSURE {
                    return Err("insufficient pressure for RECLAIM".to_string());
                }
                Ok(())
            }
            Command::Sound { unit } => {
                let entity = self
                    .entity(*unit)
                    .ok_or_else(|| "machine missing".to_string())?;
                if entity.owner != player
                    || entity.hp <= 0
                    || !matches!(entity.kind, Kind::Sounder | Kind::Skipper)
                {
                    return Err("SOUND needs a Sounder or Skipper".to_string());
                }
                if self.players[player as usize].pressure < SOUND_PRESSURE {
                    return Err("insufficient pressure for SOUND".to_string());
                }
                Ok(())
            }
            Command::Glint { unit, target } => {
                let entity = self
                    .entity(*unit)
                    .ok_or_else(|| "machine missing".to_string())?;
                if entity.owner != player || entity.hp <= 0 || entity.kind != Kind::Glinter {
                    return Err("GLINT needs a Glinter".to_string());
                }
                if entity.pos == *target {
                    return Err("GLINT needs a direction".to_string());
                }
                if self.players[player as usize].pressure < bw_content::GLINT_PRESSURE {
                    return Err("insufficient pressure for GLINT".to_string());
                }
                Ok(())
            }
            Command::Lay { unit, target } => {
                let entity = self
                    .entity(*unit)
                    .ok_or_else(|| "machine missing".to_string())?;
                if entity.owner != player
                    || entity.hp <= 0
                    || entity.kind != Kind::Salter
                    || entity.aboard.is_some()
                {
                    return Err("LAY needs a Salter".to_string());
                }
                self.lay_plan(entity.pos, *target)?;
                if self.players[player as usize].pressure < bw_content::LAY_PRESSURE_PER_ROW {
                    return Err("insufficient pressure for LAY".to_string());
                }
                Ok(())
            }
            Command::CancelResearch { building } => {
                let research = self.players[player as usize]
                    .research
                    .ok_or_else(|| "no doctrine research is active".to_string())?;
                if research.building != *building {
                    return Err("that headquarters has no active doctrine research".to_string());
                }
                let entity = self
                    .entity(*building)
                    .ok_or_else(|| "research headquarters missing".to_string())?;
                if entity.owner != player || entity.kind != Kind::Headquarters {
                    return Err("research headquarters is not controlled".to_string());
                }
                Ok(())
            }
            Command::Surrender => Ok(()),
        }
    }

    fn validate_units(&self, player: u8, ids: &[u32]) -> Result<(), String> {
        if ids.is_empty() || ids.len() > MAX_SELECTION {
            return Err("unit selection size is invalid".to_string());
        }
        let mut previous = None;
        for id in ids {
            if previous.is_some_and(|prior| *id <= prior) {
                return Err("unit IDs must be sorted and unique".to_string());
            }
            previous = Some(*id);
            let entity = self
                .entity(*id)
                .ok_or_else(|| format!("entity {id} missing"))?;
            if entity.aboard.is_some() {
                return Err(
                    "a machine aboard a transport takes no orders; unload first".to_string()
                );
            }
            if entity.owner != player || entity.hp <= 0 {
                return Err(format!("entity {id} is not controlled"));
            }
        }
        Ok(())
    }

    fn validate_target(&self, target: Pos) -> Result<(), String> {
        if !target.valid(self.map.width, self.map.height) {
            return Err("target is outside map".to_string());
        }
        Ok(())
    }

    fn apply_pending_commands(&mut self) {
        self.pending
            .sort_by_key(|item| (item.execute_tick, item.player, item.sequence));
        let current = self.tick;
        let mut ready = Vec::new();
        let mut future = Vec::new();
        for item in self.pending.drain(..) {
            if item.execute_tick <= current {
                ready.push(item);
            } else {
                future.push(item);
            }
        }
        self.pending = future;
        for item in ready {
            // Submission checks provide immediate UI feedback, while this
            // second check closes same-tick races between queued purchases,
            // deployment, capture, and other state-dependent commands.
            let result = self
                .validate_submission(item.player, &item.command)
                .and_then(|_| self.apply_command(item.player, item.command.clone()));
            if let Some(record) = self.command_log.get_mut(item.log_index) {
                record.applied = Some(result.is_ok());
                if let Err(reason) = &result {
                    record.reason = Some(reason.clone());
                }
            }
            match result {
                Ok(()) => self.events.push(Event::text(
                    self.tick,
                    EventKind::CommandAccepted,
                    Some(item.player),
                    "command applied",
                )),
                Err(reason) => self.events.push(Event::text(
                    self.tick,
                    EventKind::CommandRejected,
                    Some(item.player),
                    reason,
                )),
            }
        }
    }

    fn apply_command(&mut self, player: u8, command: Command) -> Result<(), String> {
        match command {
            Command::Move {
                units,
                target,
                queued,
            } => {
                if !queued {
                    self.drop_build_queues(&units);
                }
                self.set_move_orders(player, &units, target, false, queued)
            }
            Command::AttackMove {
                units,
                target,
                queued,
            } => {
                if !queued {
                    self.drop_build_queues(&units);
                }
                self.set_move_orders(player, &units, target, true, queued)
            }
            Command::Attack { units, target } => {
                self.drop_build_queues(&units);
                let aim = self.entity(target).map(|entity| entity.pos);
                for id in units {
                    // A deployed gun with the target out of its reach packs
                    // and goes (rules 19): in trial 10 a right-click sent
                    // deployed Looms an attack they could never carry out.
                    let out_of_reach = aim.is_some_and(|aim| {
                        self.entity(id).is_some_and(|entity| {
                            entity.pos.distance_sq(aim)
                                > i64::from(self.weapon_range(entity)).pow(2)
                        })
                    });
                    if out_of_reach
                        && (self.kept_deployed(id)
                            || self.pack_then(player, id, Order::Attack { target }))
                    {
                        continue;
                    }
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Attack { target };
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                    }
                }
                Ok(())
            }
            Command::QueueGather { units, resource } => {
                // A worker on a site keeps it and gathers after; the rest
                // gather now.
                let (later, now): (Vec<u32>, Vec<u32>) =
                    units.into_iter().partition(|id| self.has_open_sites(*id));
                for id in later {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.after_build = Some(Order::Gather { resource });
                    }
                }
                if now.is_empty() {
                    return Ok(());
                }
                self.apply_command(
                    player,
                    Command::Gather {
                        units: now,
                        resource,
                    },
                )
            }
            Command::Gather { units, resource } => {
                self.drop_build_queues(&units);
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Gather { resource };
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        entity.gather_ticks = 0;
                    }
                }
                Ok(())
            }
            Command::Build {
                worker,
                kind,
                pos,
                queued,
            } => self.start_build(player, worker, kind, pos, queued),
            Command::Train { building, kind } => self.start_train(player, building, kind),
            Command::Cancel { building } => self.cancel_build_or_queue(player, building),
            Command::Rally { building, pos } => {
                if let Some(entity) = self.entity_mut(building) {
                    entity.rally = Some(pos);
                    Ok(())
                } else {
                    Err("building missing".to_string())
                }
            }
            Command::Stop { units } => {
                self.drop_build_queues(&units);
                for id in units {
                    self.clear_order(id);
                }
                Ok(())
            }
            Command::Hold { units } => {
                self.drop_build_queues(&units);
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Hold;
                        entity.path.clear();
                        entity.waypoints.clear();
                        entity.after_pack = None;
                    }
                }
                Ok(())
            }
            Command::Deploy { units } => {
                if units.iter().any(|id| {
                    self.entity(*id)
                        .is_some_and(|entity| entity.surge_remaining > 0)
                }) {
                    return Err("Surge is active; wait for it to end".to_string());
                }
                for id in units {
                    let siege = self.players[player as usize]
                        .upgrades
                        .contains(&Upgrade::Siege);
                    if self
                        .entity(id)
                        .is_some_and(|entity| !entity.deployed && !self.may_deploy_here(entity))
                    {
                        continue;
                    }
                    let target = if let Some(entity) = self.entity_mut(id) {
                        entity.deploy_target = !entity.deployed;
                        entity.deploy_remaining = transition_ticks(entity.kind, siege);
                        entity.after_pack = None;
                        entity.order = Order::Deploy;
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        entity.waypoints.clear();
                        entity.deploy_target
                    } else {
                        false
                    };
                    self.events.push(Event::simple(
                        self.tick,
                        EventKind::Deploy,
                        Some(player),
                        Some(id),
                        if target { "deploying" } else { "packing" },
                    ));
                }
                Ok(())
            }
            Command::Capture { units } => {
                self.drop_build_queues(&units);
                for id in units {
                    // A deployed machine packs first and then goes, as a
                    // move does (rules 19): in trial 10 G on deployed
                    // Heliostats said "Capture order sent" and none moved.
                    // One kept deployed stays and keeps firing.
                    if self.kept_deployed(id) || self.pack_then(player, id, Order::Capture) {
                        continue;
                    }
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Capture;
                        entity.path.clear();
                        entity.path_index = 0;
                    }
                }
                Ok(())
            }
            Command::SetDeployed { units, deployed } => {
                self.set_deployed(player, &units, deployed);
                Ok(())
            }
            Command::KeepDeployed { units, keep } => {
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.keep_deployed = keep;
                    }
                }
                Ok(())
            }
            Command::Board { units, transport } => {
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Board { transport };
                        entity.after_build = None;
                        entity.waypoints.clear();
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        entity.after_pack = None;
                    }
                }
                Ok(())
            }
            Command::Unload { transport } => {
                let landed = self.unload(transport);
                if landed == 0 {
                    return Err("no free ground beside the transport".to_string());
                }
                Ok(())
            }
            Command::Flood => {
                if self.players[player as usize].pressure < FLOOD_PRESSURE {
                    return Err("insufficient pressure for the flood at execution".to_string());
                }
                self.players[player as usize].pressure -= FLOOD_PRESSURE;
                self.players[player as usize].pressure_spent = self.players[player as usize]
                    .pressure_spent
                    .saturating_add(u64::from(FLOOD_PRESSURE));
                self.gate.warning_until =
                    Some(self.tick.saturating_add(FLOOD_WARNING_TICKS as u64));
                self.gate.switch_target = None;
                self.gate.flood_pending = true;
                self.events.push(Event::text(
                    self.tick,
                    EventKind::GateWarning,
                    Some(player),
                    "flood in five seconds",
                ));
                Ok(())
            }
            Command::SwitchGate | Command::SetTide { .. } => {
                if self.players[player as usize].pressure < SWITCH_PRESSURE {
                    return Err("insufficient pressure for the switch at execution".to_string());
                }
                self.players[player as usize].pressure -= SWITCH_PRESSURE;
                self.players[player as usize].pressure_spent = self.players[player as usize]
                    .pressure_spent
                    .saturating_add(u64::from(SWITCH_PRESSURE));
                self.gate.warning_until = Some(self.tick.saturating_add(GATE_WARNING_TICKS as u64));
                let arm = match command {
                    Command::SetTide { arm } => arm,
                    // From the neutral tide the switch opens arm 0 (the north).
                    _ if self.gate.tide == Tide::Open => self.next_arm(),
                    _ => Arm(0),
                };
                self.gate.flood_pending = false;
                self.gate.switch_target = Some(arm);
                self.events.push(Event::text(
                    self.tick,
                    EventKind::GateWarning,
                    self.gate.owner,
                    "sluice switch warning",
                ));
                Ok(())
            }
            Command::Repair { units, target } => {
                // A repair order on an unfinished site resumes its construction:
                // the first worker becomes the site's builder and every ordered
                // worker walks to the site. Only the builder advances the work.
                self.drop_build_queues(&units);
                let unfinished = self
                    .entity(target)
                    .is_some_and(|building| building.build_remaining > 0);
                if unfinished {
                    if let Some(&first) = units.first()
                        && let Some(building) = self.entity_mut(target)
                    {
                        building.builder = Some(first);
                    }
                    for id in units {
                        if let Some(entity) = self.entity_mut(id) {
                            entity.order = Order::Build { target };
                            entity.path.clear();
                            entity.path_index = 0;
                        }
                    }
                    return Ok(());
                }
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Repair { target };
                        entity.path.clear();
                        entity.path_index = 0;
                    }
                }
                Ok(())
            }
            Command::SetFormation { units, formation } => {
                for id in units {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.formation = formation;
                    }
                }
                Ok(())
            }
            Command::Deliver { units, target } => {
                for id in units {
                    let Some(worker) = self.entity(id) else {
                        continue;
                    };
                    // An empty worker at its wreck has nothing to hand in.
                    if worker.carried == 0 && matches!(worker.order, Order::Gather { .. }) {
                        continue;
                    }
                    let resource = match worker.order {
                        Order::Gather { resource } => Some(resource),
                        Order::Deliver { resource, .. } => resource,
                        _ => None,
                    };
                    self.drop_build_queues(&[id]);
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Deliver { target, resource };
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        entity.waypoints.clear();
                        entity.gather_ticks = 0;
                        entity.after_pack = None;
                    }
                }
                Ok(())
            }
            Command::Recycle { units } => {
                self.drop_build_queues(&units);
                for id in units {
                    let Some(worker) = self.entity(id) else {
                        continue;
                    };
                    let Some(target) = self.recycle_yard(player, worker.pos) else {
                        continue;
                    };
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Recycle { target };
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.waypoints.clear();
                    }
                }
                Ok(())
            }
            Command::Face { units, target } => self.set_face_orders(player, &units, target),
            Command::Surge { units } => self.start_surge(player, &units),
            Command::Research { building, doctrine } => {
                self.start_research(player, building, doctrine)
            }
            Command::CancelResearch { building } => self.cancel_research(player, building),
            Command::Upgrade { building, upgrade } => self.start_upgrade(player, building, upgrade),
            Command::CancelUpgrade { building } => self.cancel_upgrade(player, building),
            Command::Vent { building } => self.start_vent(player, building),
            Command::Reclaim { building } => self.start_reclaim(player, building),
            Command::Sound { unit } => self.start_sound(player, unit),
            Command::Glint { unit, target } => self.start_glint(player, unit, target),
            Command::Lay { unit, target } => self.start_lay(player, unit, target),
            Command::Surrender if self.players.len() == 2 => {
                self.outcome = Some(Outcome::Victory(1 - player));
                self.events.push(Event::text(
                    self.tick,
                    EventKind::Victory,
                    Some(1 - player),
                    "surrender",
                ));
                Ok(())
            }
            Command::Surrender => {
                // With three or more seats a surrender knocks this seat out
                // and the match goes on (the last seat standing wins).
                self.eliminate_seat(player, "SURRENDERED");
                self.settle_last_standing();
                Ok(())
            }
        }
    }

    fn set_move_orders(
        &mut self,
        player: u8,
        ids: &[u32],
        target: Pos,
        attack: bool,
        queued: bool,
    ) -> Result<(), String> {
        // A deployed specialist packs first and then goes with the rest:
        // the order waits on it, so a group is never split by a move.
        let siege = self.players[player as usize]
            .upgrades
            .contains(&Upgrade::Siege);
        let lane_revision = self.gate.lane_revision;
        let mut sorted_ids = ids.to_vec();
        sorted_ids.sort_unstable();
        let centroid = sorted_ids.iter().filter_map(|id| self.entity(*id)).fold(
            (0i64, 0i64, 0i64),
            |(sum_x, sum_y, count), entity| {
                (
                    sum_x.saturating_add(i64::from(entity.pos.x)),
                    sum_y.saturating_add(i64::from(entity.pos.y)),
                    count.saturating_add(1),
                )
            },
        );
        let direction = direction_octant(
            i32::try_from(i64::from(target.x).saturating_sub(centroid.0 / centroid.2.max(1)))
                .unwrap_or_default(),
            i32::try_from(i64::from(target.y).saturating_sub(centroid.1 / centroid.2.max(1)))
                .unwrap_or_default(),
        );
        // Guns in front, specialists behind: the rearmost slots of the lead
        // machine's formation go to the Bulwarks, Looms and Caissons, so an
        // attack-move never walks the artillery in first.  A group without
        // specialists keeps its slots in id order, as before.
        let count = sorted_ids.len();
        let lead_formation = sorted_ids
            .first()
            .and_then(|id| self.entity(*id))
            .map(|entity| entity.formation)
            .unwrap_or_default();
        let (forward_x, forward_y) = octant_vector(direction);
        let mut rear_first: Vec<usize> = (0..count).collect();
        rear_first.sort_by_key(|slot| {
            let (ox, oy) = formation_offset(lead_formation, *slot, count, direction);
            (
                i64::from(ox) * i64::from(forward_x) + i64::from(oy) * i64::from(forward_y),
                std::cmp::Reverse(*slot),
            )
        });
        let (specialists, others): (Vec<u32>, Vec<u32>) =
            sorted_ids.iter().copied().partition(|id| {
                self.entity(*id)
                    .is_some_and(|entity| is_specialist(entity.kind))
            });
        let mut assigned: Vec<(u32, usize)> = specialists
            .iter()
            .copied()
            .zip(rear_first.iter().copied())
            .collect();
        let mut front_slots: Vec<usize> = rear_first[specialists.len().min(count)..].to_vec();
        front_slots.sort_unstable();
        assigned.extend(others.iter().copied().zip(front_slots));
        assigned.sort_unstable();
        // An attack-move holds the group to its slowest machine: the wipes
        // in the seventh trial came from fast machines arriving alone.
        let pace = if attack {
            sorted_ids
                .iter()
                .filter_map(|id| self.entity(*id))
                .filter(|entity| !entity.kind.is_building())
                .map(|entity| spec(entity.kind).speed)
                .min()
                .unwrap_or(0)
        } else {
            0
        };
        let mut destinations = Vec::with_capacity(count);
        for (id, slot) in assigned {
            let (tx, ty) = target.cell_xy();
            let formation = self
                .entity(id)
                .map(|entity| entity.formation)
                .unwrap_or_default();
            let offset = formation_offset(formation, slot, count, direction);
            let slot_target = Pos::cell(tx + offset.0, ty + offset.1);
            if !slot_target.valid(self.map.width, self.map.height) {
                return Err("formation destination outside map".to_string());
            }
            destinations.push((id, slot_target));
        }
        let fighting: Vec<u32> = if attack {
            destinations
                .iter()
                .filter(|(id, _)| {
                    self.entity(*id).is_some_and(|entity| {
                        entity.deployed
                            && entity.deploy_remaining == 0
                            && matches!(entity.kind, Kind::Loom | Kind::Bulwark | Kind::Heliostat)
                            && self.enemy_in_reach(entity)
                    })
                })
                .map(|(id, _)| *id)
                .collect()
        } else {
            Vec::new()
        };
        // A queued move for a worker on a site waits for the site (rules
        // 19): its waypoints are only walked by a machine on the move.
        let builders: Vec<u32> = if queued {
            ids.iter()
                .copied()
                .filter(|id| self.has_open_sites(*id))
                .collect()
        } else {
            Vec::new()
        };
        let layout = self.map.layout();
        for (id, slot_target) in destinations {
            if let Some(entity) = self.entity_mut(id) {
                let settling = if entity.deploy_remaining > 0 {
                    entity.deploy_target
                } else {
                    entity.deployed
                };
                if is_specialist(entity.kind) && entity.keep_deployed && settling {
                    // Kept deployed: the rest of the group goes, this one
                    // stays and keeps firing (rules 14).
                    continue;
                }
                if attack && fighting.contains(&id) {
                    // A deployed gun with an enemy in reach finishes the
                    // fight before it packs (rules 17): in the eighth and
                    // ninth trials an attack-move packed a Bulwark line
                    // under fire.  It follows once nothing is in reach.
                    entity.after_pack = Some(Order::AttackMove {
                        target: slot_target,
                    });
                    continue;
                }
                entity.pace = pace;
                let order = if attack {
                    Order::AttackMove {
                        target: slot_target,
                    }
                } else {
                    Order::Move {
                        target: slot_target,
                    }
                };
                if is_specialist(entity.kind) && (entity.deployed || entity.deploy_remaining > 0) {
                    if entity.deploy_remaining == 0 || entity.deploy_target {
                        entity.deploy_target = false;
                        entity.deploy_remaining = transition_ticks(entity.kind, siege);
                    }
                    entity.order = Order::Deploy;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.waypoints.clear();
                    entity.after_pack = Some(order);
                    continue;
                }
                if queued && builders.contains(&id) && entity.after_build.is_none() {
                    entity.after_build = Some(order);
                } else if queued {
                    entity.waypoints.push(slot_target);
                } else {
                    entity.waypoints.clear();
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.order = if attack {
                        Order::AttackMove {
                            target: slot_target,
                        }
                    } else {
                        Order::Move {
                            target: slot_target,
                        }
                    };
                    entity.path_lane_revision = lane_revision;
                    if !is_specialist(entity.kind) || !entity.deployed {
                        entity.facing = layout.facing(player);
                    }
                }
            }
        }
        Ok(())
    }

    /// A specialist deployed (or deploying) and told to stay so.
    fn kept_deployed(&self, id: u32) -> bool {
        self.entity(id).is_some_and(|entity| {
            let settling = if entity.deploy_remaining > 0 {
                entity.deploy_target
            } else {
                entity.deployed
            };
            is_specialist(entity.kind) && settling && entity.keep_deployed
        })
    }

    /// Pack a deployed specialist (not one kept deployed) and have it take
    /// up `order` once its legs are free.  True when it was deployed and
    /// now packs; false leaves the machine to take the order at once.
    fn pack_then(&mut self, player: u8, id: u32, order: Order) -> bool {
        let siege = self.players[player as usize]
            .upgrades
            .contains(&Upgrade::Siege);
        let Some(entity) = self.entity_mut(id) else {
            return false;
        };
        let settling = if entity.deploy_remaining > 0 {
            entity.deploy_target
        } else {
            entity.deployed
        };
        if !is_specialist(entity.kind) || !settling || entity.keep_deployed {
            return false;
        }
        if entity.deploy_remaining == 0 || entity.deploy_target {
            entity.deploy_target = false;
            entity.deploy_remaining = transition_ticks(entity.kind, siege);
        }
        entity.order = Order::Deploy;
        entity.path.clear();
        entity.path_index = 0;
        entity.path_target = None;
        entity.waypoints.clear();
        entity.after_pack = Some(order);
        true
    }

    fn start_build(
        &mut self,
        player: u8,
        worker: u32,
        kind: Kind,
        pos: Pos,
        queued: bool,
    ) -> Result<(), String> {
        let content = spec(kind);
        if self.players[player as usize].salvage < content.salvage
            || self.players[player as usize].pressure < content.pressure
        {
            return Err("insufficient resources at execution".to_string());
        }
        self.players[player as usize].salvage -= content.salvage;
        self.players[player as usize].pressure -= content.pressure;
        self.players[player as usize].salvage_spent = self.players[player as usize]
            .salvage_spent
            .saturating_add(u64::from(content.salvage));
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(content.pressure));
        let id = self.spawn_building(player, kind, pos, content.build_ticks);
        if let Some(building) = self.entity_mut(id) {
            building.builder = Some(worker);
        }
        self.step_out_of_footprint(player, id);
        // A queued site waits its turn behind the one the worker is on; the
        // first site keeps its builder (rules 14).
        let busy = self.entity(worker).is_some_and(|entity| {
            matches!(entity.order, Order::Build { target } if target != id
                && self.entity(target).is_some_and(|site| site.hp > 0 && site.build_remaining > 0))
        });
        if queued && busy {
            if let Some(entity) = self.entity_mut(worker) {
                entity.build_queue.push(id);
            }
        } else {
            if let Some(entity) = self.entity_mut(worker) {
                entity.build_queue.clear();
                entity.after_build = None;
                entity.order = Order::Build { target: id };
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = Some(pos);
            }
        }
        self.events.push(Event::with_entity(
            self.tick,
            EventKind::BuildStarted,
            Some(player),
            id,
            kind.name(),
        ));
        Ok(())
    }

    /// Deploy or pack each specialist, never the other way: one already in
    /// (or turning to) the asked state is left alone, and one turning the
    /// other way turns back (rules 14).
    /// Whether a machine may deploy where it stands: a Pan only on dry
    /// ground the tide never takes (rules 18); anything else anywhere.
    fn may_deploy_here(&self, entity: &Entity) -> bool {
        if entity.kind != Kind::Pan {
            return true;
        }
        let (x, y) = entity.pos.cell_xy();
        let terrain = self.map.terrain(x, y);
        terrain.walkable() && terrain.tidal_arm().is_none()
    }

    fn set_deployed(&mut self, player: u8, ids: &[u32], deployed: bool) {
        let siege = self.players[player as usize]
            .upgrades
            .contains(&Upgrade::Siege);
        for id in ids {
            let Some(entity) = self.entity(*id) else {
                continue;
            };
            let heading = if entity.deploy_remaining > 0 {
                entity.deploy_target
            } else {
                entity.deployed
            };
            if heading == deployed || (deployed && !self.may_deploy_here(entity)) {
                continue;
            }
            if let Some(entity) = self.entity_mut(*id) {
                entity.deploy_target = deployed;
                entity.deploy_remaining = transition_ticks(entity.kind, siege);
                entity.after_pack = None;
                entity.order = Order::Deploy;
                entity.pace = 0;
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
                entity.waypoints.clear();
                entity.build_queue.clear();
                entity.after_build = None;
            }
            self.events.push(Event::simple(
                self.tick,
                EventKind::Deploy,
                Some(player),
                Some(*id),
                if deployed { "deploying" } else { "packing" },
            ));
        }
    }

    /// Drop the sites a worker was saving for later, and what it was to do
    /// after them: any new order that is not itself queued replaces them.
    fn drop_build_queues(&mut self, ids: &[u32]) {
        for id in ids {
            if let Some(entity) = self.entity_mut(*id) {
                entity.build_queue.clear();
                entity.after_build = None;
            }
        }
    }

    /// Whether a worker is building a site or saving sites for later.
    fn has_open_sites(&self, worker: u32) -> bool {
        self.entity(worker).is_some_and(|entity| {
            entity.kind.is_worker()
                && (!entity.build_queue.is_empty()
                    || matches!(entity.order, Order::Build { target } if self
                        .entity(target)
                        .is_some_and(|site| site.hp > 0 && site.build_remaining > 0)))
        })
    }

    /// A worker done with a site takes up the next one it was given, and
    /// with none left, the order it was told to take up after them
    /// (rules 19).
    fn after_site(&mut self, worker: u32) {
        if self.take_next_site(worker) {
            return;
        }
        let lane_revision = self.gate.lane_revision;
        if let Some(entity) = self.entity_mut(worker)
            && let Some(order) = entity.after_build.take()
        {
            entity.order = order;
            entity.path.clear();
            entity.path_index = 0;
            entity.path_target = None;
            entity.path_lane_revision = lane_revision;
            entity.gather_ticks = 0;
        }
    }

    /// A worker done with a site takes up the next unfinished one it was
    /// given, if any.  True when it has one.
    fn take_next_site(&mut self, worker: u32) -> bool {
        loop {
            let Some(next) = self.entity_mut(worker).and_then(|entity| {
                (!entity.build_queue.is_empty()).then(|| entity.build_queue.remove(0))
            }) else {
                return false;
            };
            let open = self
                .entity(next)
                .is_some_and(|site| site.hp > 0 && site.build_remaining > 0);
            if !open {
                continue;
            }
            if let Some(site) = self.entity_mut(next) {
                site.builder = Some(worker);
            }
            if let Some(entity) = self.entity_mut(worker) {
                entity.order = Order::Build { target: next };
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
            }
            return true;
        }
    }

    /// Whether an unfinished site has a worker on it, one saving it for
    /// later, or none at all.
    pub fn site_builder(&self, site: u32) -> SiteBuilder {
        let Some(building) = self.entity(site) else {
            return SiteBuilder::None;
        };
        let Some(worker) = building.builder.and_then(|id| self.entity(id)) else {
            return SiteBuilder::None;
        };
        if worker.hp <= 0 || worker.owner != building.owner {
            return SiteBuilder::None;
        }
        if matches!(worker.order, Order::Build { target } if target == site) {
            SiteBuilder::Building
        } else if worker.build_queue.contains(&site) && matches!(worker.order, Order::Build { .. })
        {
            SiteBuilder::Queued
        } else {
            SiteBuilder::None
        }
    }

    /// Own machines standing on a new site step off it to the nearest free
    /// cell (rules 14): a worker walking past no longer refuses a site.
    fn step_out_of_footprint(&mut self, player: u8, site: u32) {
        let Some(building) = self.entity(site).cloned() else {
            return;
        };
        let mut inside: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.owner == player
                    && entity.id != site
                    && entity.hp > 0
                    && entity.aboard.is_none()
                    && !entity.kind.is_building()
                    && navigation::steps_aside(entity)
                    && footprint_overlaps_site(&building, entity.pos.cell_xy())
            })
            .map(|entity| entity.id)
            .collect();
        inside.sort_unstable();
        for id in inside {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            let Some(free) = self.find_spawn_pos_for(entity.kind, entity.pos) else {
                continue;
            };
            if let Some(entity) = self.entity_mut(id) {
                entity.pos = free;
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
            }
        }
    }

    fn start_train(&mut self, player: u8, building: u32, kind: Kind) -> Result<(), String> {
        let content = spec(kind);
        let valid = self.entity(building).is_some_and(|entity| {
            entity.owner == player
                && entity.build_remaining == 0
                && entity.kind.produces()
                && entity.queue.len() < MAX_QUEUE
                && trains(self.players[player as usize].faction, entity.kind, kind)
        });
        if !valid {
            return Err("production building is no longer available".to_string());
        }
        if self.players[player as usize].salvage < content.salvage
            || self.players[player as usize].pressure < content.pressure
        {
            return Err("insufficient resources at execution".to_string());
        }
        self.players[player as usize].salvage -= content.salvage;
        self.players[player as usize].pressure -= content.pressure;
        self.players[player as usize].salvage_spent = self.players[player as usize]
            .salvage_spent
            .saturating_add(u64::from(content.salvage));
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(content.pressure));
        let Some(entity) = self.entity_mut(building) else {
            return Err("building missing".to_string());
        };
        entity.queue.push(Production {
            kind,
            remaining: content.build_ticks.max(1),
            started: false,
            cost_salvage: content.salvage,
            cost_pressure: content.pressure,
        });
        self.events.push(Event::with_entity(
            self.tick,
            EventKind::ProductionQueued,
            Some(player),
            building,
            kind.name(),
        ));
        Ok(())
    }

    fn cancel_build_or_queue(&mut self, player: u8, building: u32) -> Result<(), String> {
        let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.id == building)
        else {
            return Err("building missing".to_string());
        };
        if self.entities[index].build_remaining > 0 {
            let content = spec(self.entities[index].kind);
            self.players[player as usize].salvage = self.players[player as usize]
                .salvage
                .saturating_add(content.salvage.saturating_mul(3) / 4);
            self.players[player as usize].pressure = self.players[player as usize]
                .pressure
                .saturating_add(content.pressure.saturating_mul(3) / 4);
            self.entities.remove(index);
            return Ok(());
        }
        let Some(production) = self.entities[index].queue.first().cloned() else {
            return Err("nothing to cancel".to_string());
        };
        let numerator = if production.started { 3 } else { 4 };
        self.players[player as usize].salvage = self.players[player as usize]
            .salvage
            .saturating_add(production.cost_salvage.saturating_mul(numerator) / 4);
        self.players[player as usize].pressure = self.players[player as usize]
            .pressure
            .saturating_add(production.cost_pressure.saturating_mul(numerator) / 4);
        self.entities[index].queue.remove(0);
        Ok(())
    }

    fn set_face_orders(&mut self, _player: u8, ids: &[u32], target: Pos) -> Result<(), String> {
        for id in ids {
            let Some(snapshot) = self.entity(*id).cloned() else {
                return Err(format!("entity {id} disappeared before facing"));
            };
            let (target_x, target_y) = target.cell_xy();
            let (current_x, current_y) = snapshot.pos.cell_xy();
            let direction = direction_octant(
                target_x.saturating_sub(current_x),
                target_y.saturating_sub(current_y),
            );
            let Some(entity) = self.entity_mut(*id) else {
                return Err(format!("entity {id} disappeared before facing"));
            };
            entity.path.clear();
            entity.path_index = 0;
            entity.path_target = None;
            entity.waypoints.clear();
            entity.facing = direction;
            entity.order = Order::Idle;
        }
        Ok(())
    }

    /// The machines of `ids` that can surge now: own packed combat machines
    /// off cooldown.  A capture order no longer stops one (rules 14): it
    /// runs to the station and channels once the surge ends.
    fn surge_able(&self, player: u8, ids: &[u32]) -> Vec<u32> {
        ids.iter()
            .copied()
            .filter(|id| {
                self.entity(*id).is_some_and(|entity| {
                    entity.owner == player
                        && entity.hp > 0
                        && entity.aboard.is_none()
                        && is_combat_unit(entity.kind)
                        && !entity.deployed
                        && entity.deploy_remaining == 0
                        && entity.surge_remaining == 0
                        && entity.surge_cooldown == 0
                })
            })
            .collect()
    }

    fn start_surge(&mut self, player: u8, ids: &[u32]) -> Result<(), String> {
        let ids = self.surge_able(player, ids);
        if ids.is_empty() {
            return Err("selected unit is not eligible for Surge".to_string());
        }
        let ids = ids.as_slice();
        let cost = SURGE_PRESSURE.saturating_mul(ids.len() as u32);
        if self.players[player as usize].pressure < cost {
            return Err("insufficient pressure for selected Surge units".to_string());
        }
        self.players[player as usize].pressure -= cost;
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(cost));
        for id in ids {
            if let Some(entity) = self.entity_mut(*id) {
                entity.surge_remaining = SURGE_TICKS;
                entity.surge_cooldown = SURGE_COOLDOWN_TICKS;
            }
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Surge,
                player: Some(player),
                entity: Some(*id),
                other: None,
                from: None,
                to: None,
                amount: SURGE_PRESSURE as i32,
                text: "Surge activated".to_string(),
                cause: None,
            });
        }
        Ok(())
    }

    fn start_research(
        &mut self,
        player: u8,
        building: u32,
        doctrine: Doctrine,
    ) -> Result<(), String> {
        let valid_hq = self.entity(building).is_some_and(|entity| {
            entity.owner == player
                && entity.kind == Kind::Headquarters
                && entity.build_remaining == 0
                && entity.hp > 0
        });
        if !valid_hq {
            return Err("research requires a finished headquarters".to_string());
        }
        let (salvage, pressure) = self.research_price(player, doctrine)?;
        if self.players[player as usize].salvage < salvage
            || self.players[player as usize].pressure < pressure
        {
            return Err("insufficient resources for doctrine research at execution".to_string());
        }
        let tier = self.players[player as usize].doctrine_tier + 1;
        self.players[player as usize].salvage -= salvage;
        self.players[player as usize].pressure -= pressure;
        self.players[player as usize].salvage_spent = self.players[player as usize]
            .salvage_spent
            .saturating_add(u64::from(salvage));
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(pressure));
        self.players[player as usize].research = Some(Research {
            building,
            doctrine,
            remaining: if tier >= 2 {
                DOCTRINE_TIER2_TICKS
            } else {
                DOCTRINE_TICKS
            },
            tier,
        });
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::ResearchStarted,
            player: Some(player),
            entity: Some(building),
            other: None,
            from: None,
            to: None,
            amount: salvage as i32,
            text: format!("{doctrine:?} research started"),
            cause: None,
        });
        Ok(())
    }

    /// The price of the next doctrine tier, or why it cannot start.
    fn research_price(&self, player: u8, doctrine: Doctrine) -> Result<(u32, u32), String> {
        let slot = &self.players[player as usize];
        if slot.research.is_some() {
            return Err("headquarters research is already running".to_string());
        }
        match (slot.doctrine, slot.doctrine_tier) {
            (None, _) => Ok((DOCTRINE_SALVAGE, DOCTRINE_PRESSURE)),
            (Some(chosen), 1) if chosen == doctrine => {
                Ok((DOCTRINE_TIER2_SALVAGE, DOCTRINE_TIER2_PRESSURE))
            }
            (Some(_), 1) => Err("headquarters doctrine choice is already locked".to_string()),
            _ => Err("both doctrine tiers are complete".to_string()),
        }
    }

    fn upgrade_allowed(&self, player: u8, building: u32, upgrade: Upgrade) -> Result<(), String> {
        let entity = self
            .entity(building)
            .ok_or_else(|| "building missing".to_string())?;
        if entity.owner != player || entity.build_remaining > 0 || entity.hp <= 0 {
            return Err("the upgrade needs your finished building".to_string());
        }
        if entity.kind != upgrade.building() {
            return Err("this building does not offer that upgrade".to_string());
        }
        if self.players[player as usize].upgrades.contains(&upgrade) {
            return Err("that upgrade is complete".to_string());
        }
        // OVERHAUL's levels come in order (rules 21): the one before must be
        // complete, or running or queued at this same building, so a queue
        // keeps them in order and a cancel takes the last one first.
        if let Some(before) = upgrade.requires()
            && !self.players[player as usize].upgrades.contains(&before)
            && entity.upgrade.is_none_or(|job| job.upgrade != before)
            && !entity.upgrade_queue.contains(&before)
        {
            return Err(format!("{} needs {} first", upgrade.name(), before.name()));
        }
        if self.entities.iter().any(|other| {
            other.owner == player
                && (other.upgrade.is_some_and(|job| job.upgrade == upgrade)
                    || other.upgrade_queue.contains(&upgrade))
        }) {
            return Err("that upgrade is already running or queued".to_string());
        }
        if entity.upgrade.is_some() && entity.upgrade_queue.len() >= UPGRADE_QUEUE {
            return Err("the upgrade queue here is full".to_string());
        }
        Ok(())
    }

    fn start_upgrade(&mut self, player: u8, building: u32, upgrade: Upgrade) -> Result<(), String> {
        self.upgrade_allowed(player, building, upgrade)?;
        let (salvage, pressure, ticks) = upgrade.cost();
        if self.players[player as usize].salvage < salvage
            || self.players[player as usize].pressure < pressure
        {
            return Err("insufficient resources for the upgrade at execution".to_string());
        }
        self.players[player as usize].salvage -= salvage;
        self.players[player as usize].pressure -= pressure;
        self.players[player as usize].salvage_spent = self.players[player as usize]
            .salvage_spent
            .saturating_add(u64::from(salvage));
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(pressure));
        let mut queued = false;
        if let Some(entity) = self.entity_mut(building) {
            if entity.upgrade.is_some() {
                entity.upgrade_queue.push(upgrade);
                queued = true;
            } else {
                entity.upgrade = Some(UpgradeJob {
                    upgrade,
                    remaining: ticks.max(1),
                });
            }
        }
        if queued {
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::UpgradeQueued,
                Some(player),
                building,
                upgrade.name(),
            ));
            return Ok(());
        }
        self.events.push(Event::with_entity(
            self.tick,
            EventKind::UpgradeStarted,
            Some(player),
            building,
            upgrade.name(),
        ));
        Ok(())
    }

    fn cancel_upgrade(&mut self, player: u8, building: u32) -> Result<(), String> {
        // The last waiting upgrade goes first, and all of its price returns.
        let waiting = self
            .entity_mut(building)
            .filter(|entity| entity.owner == player)
            .and_then(|entity| entity.upgrade_queue.pop());
        if let Some(upgrade) = waiting {
            let (salvage, pressure, _) = upgrade.cost();
            let cap = self.players[player as usize].pressure_cap;
            let side = &mut self.players[player as usize];
            side.salvage = side.salvage.saturating_add(salvage);
            side.pressure = side.pressure.saturating_add(pressure).min(cap);
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::UpgradeCancelled,
                Some(player),
                building,
                "queued upgrade cancelled; all refunded",
            ));
            return Ok(());
        }
        let job = self
            .entity(building)
            .filter(|entity| entity.owner == player)
            .and_then(|entity| entity.upgrade)
            .ok_or_else(|| "no upgrade to cancel".to_string())?;
        let (salvage, pressure, _) = job.upgrade.cost();
        let cap = self.players[player as usize].pressure_cap;
        self.players[player as usize].salvage = self.players[player as usize]
            .salvage
            .saturating_add(salvage * 3 / 4);
        self.players[player as usize].pressure = self.players[player as usize]
            .pressure
            .saturating_add(pressure * 3 / 4)
            .min(cap);
        if let Some(entity) = self.entity_mut(building) {
            entity.upgrade = None;
        }
        self.events.push(Event::with_entity(
            self.tick,
            EventKind::UpgradeCancelled,
            Some(player),
            building,
            "upgrade cancelled; 75% refunded",
        ));
        Ok(())
    }

    fn update_upgrades(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.upgrade.is_some() && entity.hp > 0)
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let Some(job) = self.entity(id).and_then(|entity| entity.upgrade) else {
                continue;
            };
            if job.remaining > 1 {
                if let Some(entity) = self.entity_mut(id)
                    && let Some(running) = entity.upgrade.as_mut()
                {
                    running.remaining = running.remaining.saturating_sub(1);
                }
                continue;
            }
            let owner = self.entity(id).map(|entity| entity.owner).unwrap_or(0);
            let mut next = None;
            if let Some(entity) = self.entity_mut(id) {
                entity.upgrade = None;
                if !entity.upgrade_queue.is_empty() {
                    let upgrade = entity.upgrade_queue.remove(0);
                    entity.upgrade = Some(UpgradeJob {
                        upgrade,
                        remaining: upgrade.cost().2.max(1),
                    });
                    next = Some(upgrade);
                }
            }
            let list = &mut self.players[owner as usize].upgrades;
            if !list.contains(&job.upgrade) {
                list.push(job.upgrade);
                list.sort();
            }
            self.apply_upgrade_effects(owner, job.upgrade);
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::UpgradeCompleted,
                Some(owner),
                id,
                job.upgrade.name(),
            ));
            if let Some(upgrade) = next {
                self.events.push(Event::with_entity(
                    self.tick,
                    EventKind::UpgradeStarted,
                    Some(owner),
                    id,
                    upgrade.name(),
                ));
            }
        }
    }

    /// Immediate effects of a completed upgrade on machines in the field.
    fn apply_upgrade_effects(&mut self, owner: u8, upgrade: Upgrade) {
        if matches!(upgrade, Upgrade::Plate | Upgrade::Refit) || upgrade.overhaul_level().is_some()
        {
            let targets: Vec<(usize, i32)> = self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, entity)| {
                    entity.owner == owner && entity.hp > 0 && !entity.kind.is_building()
                })
                .map(|(index, entity)| (index, self.unit_max_hp(owner, entity.kind)))
                .collect();
            for (index, boosted) in targets {
                let entity = &mut self.entities[index];
                if entity.max_hp < boosted {
                    entity.hp = entity.hp.saturating_add(boosted - entity.max_hp);
                    entity.max_hp = boosted;
                }
            }
        }
    }

    fn start_vent(&mut self, player: u8, building: u32) -> Result<(), String> {
        let valid = self.entity(building).is_some_and(|entity| {
            entity.owner == player
                && entity.kind == Kind::Headquarters
                && entity.build_remaining == 0
        });
        if !valid {
            return Err("VENT needs your finished headquarters".to_string());
        }
        if self.players[player as usize].vent_remaining > 0 {
            return Err("VENT is already open".to_string());
        }
        if self.players[player as usize].pressure < VENT_PRESSURE {
            return Err("insufficient pressure for VENT at execution".to_string());
        }
        self.players[player as usize].pressure -= VENT_PRESSURE;
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(VENT_PRESSURE));
        self.players[player as usize].vent_remaining = VENT_TICKS;
        self.events.push(Event::with_entity(
            self.tick,
            EventKind::Vent,
            Some(player),
            building,
            "vent opened",
        ));
        Ok(())
    }

    /// RECLAIM: the headquarters melts pressure into salvage at once, so a
    /// side capped on pressure and short of salvage always has an income.
    fn start_reclaim(&mut self, player: u8, building: u32) -> Result<(), String> {
        let valid = self.entity(building).is_some_and(|entity| {
            entity.owner == player
                && entity.kind == Kind::Headquarters
                && entity.build_remaining == 0
        });
        if !valid {
            return Err("RECLAIM needs your finished headquarters".to_string());
        }
        if self.players[player as usize].pressure < RECLAIM_PRESSURE {
            return Err("insufficient pressure for RECLAIM at execution".to_string());
        }
        let side = &mut self.players[player as usize];
        side.pressure -= RECLAIM_PRESSURE;
        side.pressure_spent = side
            .pressure_spent
            .saturating_add(u64::from(RECLAIM_PRESSURE));
        side.salvage = side.salvage.saturating_add(RECLAIM_SALVAGE);
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::Reclaimed,
            player: Some(player),
            entity: Some(building),
            other: None,
            from: None,
            to: None,
            amount: i32::try_from(RECLAIM_SALVAGE).unwrap_or(i32::MAX),
            text: "reclaimed".to_string(),
            cause: None,
        });
        Ok(())
    }

    /// GLINT: a ray of sight from the Glinter toward `target`, GLINT_LENGTH
    /// long whatever the distance to the point.
    fn start_glint(&mut self, player: u8, unit: u32, target: Pos) -> Result<(), String> {
        let Some(entity) = self.entity(unit).cloned() else {
            return Err("machine missing".to_string());
        };
        if entity.owner != player || entity.hp <= 0 || entity.kind != Kind::Glinter {
            return Err("GLINT needs a Glinter".to_string());
        }
        let price = bw_content::GLINT_PRESSURE;
        if self.players[player as usize].pressure < price {
            return Err("insufficient pressure for GLINT at execution".to_string());
        }
        let (dx, dy) = (
            i64::from(target.x) - i64::from(entity.pos.x),
            i64::from(target.y) - i64::from(entity.pos.y),
        );
        let length = (dx * dx + dy * dy).isqrt();
        if length == 0 {
            return Err("GLINT needs a direction".to_string());
        }
        let reach = i64::from(bw_content::GLINT_LENGTH);
        let end = Pos::raw(
            (i64::from(entity.pos.x) + dx * reach / length) as i32,
            (i64::from(entity.pos.y) + dy * reach / length) as i32,
        );
        self.players[player as usize].pressure -= price;
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(price));
        if self.beacons.len() >= MAX_ENTITIES {
            self.beacons.remove(0);
        }
        self.beacons.push(Beacon {
            owner: player,
            pos: entity.pos,
            until: self.tick.saturating_add(u64::from(bw_content::GLINT_TICKS)),
            radius: bw_content::GLINT_WIDTH / 2,
            ray_to: Some(end),
        });
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::Glint,
            player: Some(player),
            entity: Some(unit),
            other: None,
            from: Some(entity.pos),
            to: Some(end),
            amount: bw_content::GLINT_LENGTH,
            text: "glinted".to_string(),
            cause: None,
        });
        Ok(())
    }

    /// Where a LAY toward `target` starts: the bank cell behind the target
    /// along the main axis from the Salter, and that step. The target must
    /// be tidal water, and walking back from it must reach walkable dry
    /// ground before anything else.
    pub fn lay_plan(&self, from: Pos, target: Pos) -> Result<(Pos, i8, i8), String> {
        let (tx, ty) = target.cell_xy();
        if self.map.terrain(tx, ty).tidal_arm().is_none() {
            return Err("LAY needs tidal water".to_string());
        }
        let (fx, fy) = from.cell_xy();
        let (ox, oy) = (tx - fx, ty - fy);
        let (dx, dy) = if ox == 0 && oy == 0 {
            return Err("LAY needs a direction".to_string());
        } else if ox.abs() >= oy.abs() {
            (ox.signum(), 0)
        } else {
            (0, oy.signum())
        };
        let (width, height) = (i32::from(self.map.width), i32::from(self.map.height));
        let (mut x, mut y) = (tx, ty);
        loop {
            x -= dx;
            y -= dy;
            if !(0..width).contains(&x) || !(0..height).contains(&y) {
                return Err("LAY needs a bank".to_string());
            }
            let terrain = self.map.terrain(x, y);
            if terrain.tidal_arm().is_some() {
                continue;
            }
            if !terrain.walkable() {
                return Err("LAY needs a bank".to_string());
            }
            return Ok((Pos::cell(x, y), dx as i8, dy as i8));
        }
    }

    fn start_lay(&mut self, player: u8, unit: u32, target: Pos) -> Result<(), String> {
        let Some(entity) = self.entity(unit).cloned() else {
            return Err("machine missing".to_string());
        };
        if entity.owner != player || entity.hp <= 0 || entity.kind != Kind::Salter {
            return Err("LAY needs a Salter".to_string());
        }
        let (head, dx, dy) = self.lay_plan(entity.pos, target)?;
        self.clear_order(unit);
        if let Some(entity) = self.entity_mut(unit) {
            entity.order = Order::Lay {
                head,
                dx,
                dy,
                rows: 0,
                progress: 0,
            };
        }
        Ok(())
    }

    /// Salters standing on the head of their causeway crust the next row.
    /// A row needs tidal water at its middle cell and LAY_PRESSURE_PER_ROW;
    /// without either, or after LAY_MAX_ROWS, the Salter stops.
    fn update_laying(&mut self) {
        let layers: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && matches!(entity.order, Order::Lay { .. }))
            .map(|entity| entity.id)
            .collect();
        for id in layers {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            let Order::Lay {
                head,
                dx,
                dy,
                rows,
                progress,
            } = entity.order
            else {
                continue;
            };
            if entity.pos.cell_xy() != head.cell_xy() || !entity.path.is_empty() {
                continue;
            }
            if u32::from(progress) + 1 < bw_content::LAY_ROW_TICKS {
                if let Some(entity) = self.entity_mut(id) {
                    entity.order = Order::Lay {
                        head,
                        dx,
                        dy,
                        rows,
                        progress: progress + 1,
                    };
                }
                continue;
            }
            let (hx, hy) = head.cell_xy();
            let (dx, dy) = (i32::from(dx), i32::from(dy));
            let (cx, cy) = (hx + dx, hy + dy);
            let owner = entity.owner as usize;
            let in_map = (0..i32::from(self.map.width)).contains(&cx)
                && (0..i32::from(self.map.height)).contains(&cy);
            let water = in_map
                && self.map.terrain(cx, cy).tidal_arm().is_some()
                && !self.is_crusted(cx, cy);
            let crossed = in_map && !water && self.depth_at(cx, cy) == Some(Depth::Dry);
            if !water
                || u32::from(rows) >= bw_content::LAY_MAX_ROWS
                || self.players[owner].pressure < bw_content::LAY_PRESSURE_PER_ROW
            {
                // An already crusted row ahead is walked onto, not stopped at.
                if crossed && u32::from(rows) < bw_content::LAY_MAX_ROWS {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Lay {
                            head: Pos::cell(cx, cy),
                            dx: dx as i8,
                            dy: dy as i8,
                            rows,
                            progress: 0,
                        };
                    }
                    continue;
                }
                self.clear_order(id);
                continue;
            }
            let price = bw_content::LAY_PRESSURE_PER_ROW;
            self.players[owner].pressure -= price;
            self.players[owner].pressure_spent = self.players[owner]
                .pressure_spent
                .saturating_add(u64::from(price));
            let half = bw_content::LAY_WIDTH_CELLS / 2;
            let width = i32::from(self.map.width);
            for offset in -half..=half {
                let (x, y) = (cx + dy.abs() * offset, cy + dx.abs() * offset);
                if !(0..width).contains(&x)
                    || !(0..i32::from(self.map.height)).contains(&y)
                    || self.map.terrain(x, y).tidal_arm().is_none()
                    || self.hull_on((x, y))
                {
                    continue;
                }
                let index = (y * width + x) as u32;
                if let Err(slot) = self.crust.binary_search(&index) {
                    self.crust.insert(slot, index);
                }
            }
            self.gate.lane_revision = self.gate.lane_revision.saturating_add(1);
            self.lane_revision = self.gate.lane_revision;
            let next = Pos::cell(cx, cy);
            if let Some(entity) = self.entity_mut(id) {
                entity.order = Order::Lay {
                    head: next,
                    dx: dx as i8,
                    dy: dy as i8,
                    rows: rows + 1,
                    progress: 0,
                };
            }
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Laid,
                player: Some(entity.owner),
                entity: Some(id),
                other: None,
                from: Some(head),
                to: Some(next),
                amount: i32::from(rows) + 1,
                text: "laid".to_string(),
                cause: None,
            });
        }
    }

    /// Whether a hull floats on this cell: the Salter does not crust under
    /// a boat.
    fn hull_on(&self, cell: (i32, i32)) -> bool {
        self.entities.iter().any(|entity| {
            entity.hp > 0
                && entity.aboard.is_none()
                && matches!(spec(entity.kind).movement, Movement::Hull)
                && entity.pos.cell_xy() == cell
        })
    }

    fn start_sound(&mut self, player: u8, unit: u32) -> Result<(), String> {
        let Some(entity) = self.entity(unit).cloned() else {
            return Err("machine missing".to_string());
        };
        if entity.owner != player
            || entity.hp <= 0
            || !matches!(entity.kind, Kind::Sounder | Kind::Skipper)
        {
            return Err("SOUND needs a Sounder or Skipper".to_string());
        }
        if self.players[player as usize].pressure < SOUND_PRESSURE {
            return Err("insufficient pressure for SOUND at execution".to_string());
        }
        self.players[player as usize].pressure -= SOUND_PRESSURE;
        self.players[player as usize].pressure_spent = self.players[player as usize]
            .pressure_spent
            .saturating_add(u64::from(SOUND_PRESSURE));
        if self.beacons.len() >= MAX_ENTITIES {
            self.beacons.remove(0);
        }
        self.beacons.push(Beacon {
            owner: player,
            pos: entity.pos,
            until: self.tick.saturating_add(u64::from(SOUND_TICKS)),
            radius: SOUND_RADIUS,
            ray_to: None,
        });
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::Sound,
            player: Some(player),
            entity: Some(unit),
            other: None,
            from: Some(entity.pos),
            to: Some(entity.pos),
            amount: SOUND_RADIUS,
            text: "sounded".to_string(),
            cause: None,
        });
        Ok(())
    }

    fn cancel_research(&mut self, player: u8, building: u32) -> Result<(), String> {
        let Some(research) = self.players[player as usize].research else {
            return Err("no doctrine research is active".to_string());
        };
        if research.building != building
            || !self
                .entity(building)
                .is_some_and(|entity| entity.owner == player && entity.kind == Kind::Headquarters)
        {
            return Err("that headquarters has no active doctrine research".to_string());
        }
        self.players[player as usize].salvage = self.players[player as usize]
            .salvage
            .saturating_add(DOCTRINE_REFUND_SALVAGE);
        let cap = self.players[player as usize].pressure_cap;
        self.players[player as usize].pressure = self.players[player as usize]
            .pressure
            .saturating_add(DOCTRINE_REFUND_PRESSURE)
            .min(cap);
        self.players[player as usize].research = None;
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::ResearchCancelled,
            player: Some(player),
            entity: Some(building),
            other: None,
            from: None,
            to: None,
            amount: DOCTRINE_REFUND_SALVAGE as i32,
            text: "doctrine research cancelled; 75% refunded".to_string(),
            cause: None,
        });
        Ok(())
    }
}

const DOCTRINE_REFUND_SALVAGE: u32 = DOCTRINE_SALVAGE * 3 / 4;
const DOCTRINE_REFUND_PRESSURE: u32 = DOCTRINE_PRESSURE * 3 / 4;

fn formation_offset(formation: Formation, slot: usize, count: usize, direction: u8) -> (i32, i32) {
    const OFFSETS: [(i32, i32); 9] = [
        (0, 0),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ];
    match formation {
        Formation::Compact | Formation::Loose => {
            let spacing = if formation == Formation::Loose { 3 } else { 1 };
            let (x, y) = if slot < OFFSETS.len() {
                OFFSETS[slot]
            } else {
                // Keep the familiar nine slots, then grow into a bounded
                // five-column grid.  Every selected ID receives a unique
                // deterministic slot through the first 128 entities.
                let extra = slot - OFFSETS.len();
                (extra as i32 % 5 - 2, 2 + extra as i32 / 5)
            };
            (x.saturating_mul(spacing), y.saturating_mul(spacing))
        }
        Formation::Line => {
            // Eight lateral columns keep a large selection on the basin
            // without producing a single off-map line.  Additional ranks
            // stack along travel direction.
            let rank = slot as i32 % 8 - (count.min(8) as i32 / 2);
            let row = slot as i32 / 8;
            let (lateral_x, lateral_y) = match direction & 7 {
                0 | 4 => (1, 0),
                2 | 6 => (0, 1),
                1 | 5 => (1, 1),
                _ => (1, -1),
            };
            let (forward_x, forward_y) = octant_vector(direction);
            (
                rank.saturating_mul(lateral_x)
                    .saturating_add(row.saturating_mul(forward_x)),
                rank.saturating_mul(lateral_y)
                    .saturating_add(row.saturating_mul(forward_y)),
            )
        }
    }
}

impl World {
    fn update_pressure(&mut self) {
        let mut condenser_count = vec![0u32; self.players.len()];
        let mut pan_count = vec![0u32; self.players.len()];
        for entity in &self.entities {
            if entity.kind == Kind::Condenser && entity.hp > 0 && entity.build_remaining == 0 {
                condenser_count[entity.owner as usize] =
                    condenser_count[entity.owner as usize].saturating_add(1);
            }
            if entity.kind == Kind::Pan
                && entity.hp > 0
                && entity.deployed
                && entity.deploy_remaining == 0
            {
                pan_count[entity.owner as usize] =
                    pan_count[entity.owner as usize].saturating_add(1);
            }
        }
        for (player, count) in condenser_count.into_iter().enumerate() {
            // A seat knocked out of the match earns nothing more (rules
            // 19): its bar kept filling and spilling into salvage.
            if self.is_eliminated(player as u8) {
                continue;
            }
            // Quarter-pressure units a tick: the headquarters gives one a
            // second, each condenser two more, the sluice a quarter, and
            // Bleed Valves one more.
            let mut rate = 4u32.saturating_add(count.saturating_mul(8));
            if self.gate.owner == Some(player as u8) {
                rate = rate.saturating_add(SLUICE_PRESSURE_PER_MINUTE / 15);
            }
            if self.players[player]
                .upgrades
                .contains(&Upgrade::BleedValves)
            {
                rate = rate.saturating_add(4);
            }
            let threshold = TICK_HZ as u32 * 4;
            self.players[player].pressure_remainder =
                self.players[player].pressure_remainder.saturating_add(rate);
            // Deployed Pans boil their own pressure, counted apart so the
            // other income keeps its exact rate (rules 18).
            let mut pan_amount = 0;
            if pan_count[player] > 0 {
                let minute = TICK_HZ as u32 * 60;
                let side = &mut self.players[player];
                side.pan_remainder = side.pan_remainder.saturating_add(
                    pan_count[player].saturating_mul(bw_content::PAN_PRESSURE_PER_MINUTE),
                );
                pan_amount = side.pan_remainder / minute;
                side.pan_remainder %= minute;
            }
            if self.players[player].pressure_remainder >= threshold || pan_amount > 0 {
                let amount = self.players[player].pressure_remainder / threshold + pan_amount;
                let cap = self.players[player].pressure_cap;
                let side = &mut self.players[player];
                let total = side.pressure.saturating_add(amount);
                side.pressure = total.min(cap);
                side.pressure_remainder %= threshold;
                // Income over a full bar becomes salvage at a slow rate
                // (rules 15) instead of vanishing.
                side.overflow = side.overflow.saturating_add(total.saturating_sub(cap));
                if side.overflow >= OVERFLOW_PRESSURE {
                    let lots = side.overflow / OVERFLOW_PRESSURE;
                    side.overflow %= OVERFLOW_PRESSURE;
                    side.salvage = side
                        .salvage
                        .saturating_add(lots.saturating_mul(OVERFLOW_SALVAGE));
                }
            }
        }
    }

    /// Salvage that needs no wreck (rules 20): each standing headquarters
    /// yields HQ_SALVAGE_PER_MINUTE, and the station's owner gains
    /// STATION_SALVAGE_PER_MINUTE while it owns it.
    fn update_salvage_trickle(&mut self) {
        let minute = TICK_HZ as u32 * 60;
        let mut rate = vec![0u32; self.players.len()];
        for entity in &self.entities {
            if entity.kind == Kind::Headquarters
                && entity.hp > 0
                && entity.build_remaining == 0
                && let Some(slot) = rate.get_mut(entity.owner as usize)
            {
                *slot = slot.saturating_add(HQ_SALVAGE_PER_MINUTE);
            }
        }
        if let Some(slot) = self
            .gate
            .owner
            .and_then(|owner| rate.get_mut(owner as usize))
        {
            *slot = slot.saturating_add(STATION_SALVAGE_PER_MINUTE);
        }
        for (player, rate) in rate.into_iter().enumerate() {
            if rate == 0 || self.is_eliminated(player as u8) {
                continue;
            }
            let side = &mut self.players[player];
            side.salvage_remainder = side.salvage_remainder.saturating_add(rate);
            side.salvage = side.salvage.saturating_add(side.salvage_remainder / minute);
            side.salvage_remainder %= minute;
        }
    }

    fn update_research(&mut self) {
        for player_index in 0..self.players.len() {
            let Some(research) = self.players[player_index].research else {
                continue;
            };
            if research.remaining > 1 {
                if let Some(active) = self.players[player_index].research.as_mut() {
                    active.remaining = active.remaining.saturating_sub(1);
                }
                continue;
            }
            self.players[player_index].research = None;
            self.players[player_index].doctrine = Some(research.doctrine);
            self.players[player_index].doctrine_tier = research.tier.max(1);
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::ResearchCompleted,
                player: Some(player_index as u8),
                entity: Some(research.building),
                other: None,
                from: None,
                to: None,
                amount: 0,
                text: format!("{:?} doctrine completed", research.doctrine),
                cause: None,
            });
        }
    }

    fn update_surge(&mut self) {
        for entity in &mut self.entities {
            entity.surge_remaining = entity.surge_remaining.saturating_sub(1);
            entity.surge_cooldown = entity.surge_cooldown.saturating_sub(1);
        }
        for player in &mut self.players {
            player.vent_remaining = player.vent_remaining.saturating_sub(1);
        }
    }

    /// What one load holds: workers carry five, Hauling adds three and its
    /// second tier three more, Cranes two; a Dredger carries eight.
    fn carry_for(&self, player: u8, kind: Kind) -> u32 {
        let slot = &self.players[player as usize];
        let mut carry = if kind == Kind::Dredger { 8 } else { 5 };
        if slot.doctrine == Some(Doctrine::Hauling) {
            carry += 3;
            if slot.doctrine_tier >= 2 {
                carry += 3;
            }
        }
        if slot.upgrades.contains(&Upgrade::Cranes) {
            carry += 2;
        }
        carry
    }

    fn attack_cooldown(&self, entity: &Entity) -> u32 {
        let base = spec(entity.kind).cooldown;
        let player = &self.players[entity.owner as usize];
        let mut cooldown =
            if player.doctrine == Some(Doctrine::FireControl) && is_combat_unit(entity.kind) {
                base.saturating_mul(85).saturating_add(99) / 100
            } else {
                base
            };
        if player.vent_remaining > 0 && is_combat_unit(entity.kind) {
            cooldown = cooldown
                .saturating_mul(VENT_COOLDOWN_PERCENT)
                .saturating_add(99)
                / 100;
        }
        cooldown.max(1)
    }

    fn update_construction(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.build_remaining > 0)
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let Some(building) = self.entity(id).cloned() else {
                continue;
            };
            let Some(builder_id) = building.builder else {
                continue;
            };
            let building_pos = building.pos;
            let building_owner = building.owner;
            let build_target = self.interaction_target(
                &building,
                self.entity(builder_id)
                    .map_or(building_pos, |worker| worker.pos),
            );
            let builder_ready = self.entity(builder_id).is_some_and(|worker| {
                worker.owner == building_owner
                    && worker.hp > 0
                    && worker.kind.is_worker()
                    && matches!(worker.order, Order::Build { target } if target == id)
                    && worker.pos.distance_sq(build_target) <= i64::from(FP).pow(2)
            });
            if !builder_ready {
                continue;
            }
            let completed = if let Some(entity) = self.entity_mut(id) {
                entity.build_remaining = entity.build_remaining.saturating_sub(1);
                let total = spec(entity.kind).build_ticks.max(1);
                let done = total.saturating_sub(entity.build_remaining);
                entity.hp = (entity.max_hp / 4)
                    .saturating_add((entity.max_hp.saturating_mul(done as i32)) / total as i32)
                    .min(entity.max_hp)
                    .max(1);
                done >= total
            } else {
                false
            };
            if completed {
                if let Some(entity) = self.entity_mut(builder_id) {
                    entity.order = Order::Idle;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                }
                self.after_site(builder_id);
                if let Some(entity) = self.entity(id) {
                    self.events.push(Event::with_entity(
                        self.tick,
                        EventKind::BuildCompleted,
                        Some(entity.owner),
                        entity.id,
                        entity.kind.name(),
                    ));
                }
            }
        }
    }

    fn update_production(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.build_remaining == 0 && !entity.queue.is_empty())
            .map(|entity| entity.id)
            .collect();
        for building_id in ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == building_id)
            else {
                continue;
            };
            if self.entities[index].queue.is_empty() {
                continue;
            }
            let owner = self.entities[index].owner;
            if self.players[owner as usize]
                .research
                .is_some_and(|research| {
                    research.building == building_id
                        && self.entities[index].kind == Kind::Headquarters
                })
            {
                // Research pauses worker production at this headquarters but
                // leaves the queue intact.  (Works queues are independent.)
                continue;
            }
            self.entities[index].queue[0].started = true;
            self.entities[index].queue[0].remaining =
                self.entities[index].queue[0].remaining.saturating_sub(1);
            if self.entities[index].queue[0].remaining > 0 {
                continue;
            }
            let production = self.entities[index].queue.remove(0);
            let kind = production.kind;
            let crew_cost = spec(kind).crew;
            if self.players[owner as usize].crew.saturating_add(crew_cost)
                > self.players[owner as usize].cap
            {
                self.entities[index].queue.insert(
                    0,
                    Production {
                        remaining: 1,
                        started: true,
                        ..production
                    },
                );
                continue;
            }
            let Some(pos) = self.find_spawn_pos_for(kind, self.entities[index].pos) else {
                self.entities[index].queue.insert(
                    0,
                    Production {
                        remaining: 1,
                        started: true,
                        ..production
                    },
                );
                continue;
            };
            let unit_id = self.spawn_unit(owner, kind, pos);
            self.players[owner as usize].crew =
                self.players[owner as usize].crew.saturating_add(crew_cost);
            let rally = self.entities[index].rally.or_else(|| {
                let building = self.entities[index].clone();
                self.muster_point(kind, &building, pos)
            });
            let order = self.spawn_order(owner, kind, pos, rally);
            if let Some(unit) = self.entity_mut(unit_id) {
                unit.order = order;
            }
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::ProductionCompleted,
                Some(owner),
                unit_id,
                kind.name(),
            ));
        }
    }

    /// Where a new combat machine with no rally walks: three cells out from
    /// the door, away from the building, so a base's machines no longer
    /// crowd its door and hide it from clicks (rules 14).  Workers keep
    /// their own rules; a hull stays on its water.  The practice AI's
    /// machines stay at the door, where its scripted plan looks for them.
    fn muster_point(&self, kind: Kind, building: &Entity, spawn: Pos) -> Option<Pos> {
        // The practice AI orders every new machine itself; its plan is
        // tuned to find them at the door.
        let practice_ai = (self.ai_enabled || self.replaying_ai) && building.owner != 0;
        if practice_ai || kind.gathers() || matches!(spec(kind).movement, Movement::Hull) {
            return None;
        }
        let (bx, by) = building.pos.cell_xy();
        let size = spec(building.kind).footprint.max(1);
        let (sx, sy) = spawn.cell_xy();
        // Twice the offset from the footprint's centre, in half cells.
        let dx = (2 * sx + 1 - (2 * bx + size)).signum();
        let dy = (2 * sy + 1 - (2 * by + size)).signum();
        let (dx, dy) = if (dx, dy) == (0, 0) { (0, 1) } else { (dx, dy) };
        let wanted = (sx + dx * MUSTER_CELLS, sy + dy * MUSTER_CELLS);
        let mut best: Option<(i64, i32, i32)> = None;
        for oy in -1..=1 {
            for ox in -1..=1 {
                let cell = (wanted.0 + ox, wanted.1 + oy);
                if !Pos::cell(cell.0, cell.1).valid(self.map.width, self.map.height)
                    || !self.cell_walkable_for(kind, cell)
                    || self.map.terrain(cell.0, cell.1).is_tidal()
                {
                    continue;
                }
                let distance = Pos::cell(cell.0, cell.1).distance_sq(Pos::cell(wanted.0, wanted.1));
                if best.is_none_or(|(d, y, x)| (distance, cell.1, cell.0) < (d, y, x)) {
                    best = Some((distance, cell.1, cell.0));
                }
            }
        }
        best.map(|(_, y, x)| Pos::cell(x, y))
    }

    /// The first order of a machine leaving production.  A rally point on
    /// or beside a salvage wreck sends a new worker to gather there and any
    /// other rally is a move.  A worker with no rally joins the nearest wreck
    /// that still holds salvage within `AUTO_GATHER_RADIUS`, so a base never
    /// parks its new workers at the door.  A combat machine with no rally
    /// stands where it was built.
    ///
    /// A rally wreck inside ground where this side's workers were hit lately
    /// is not walked into: the new worker takes the nearest safe wreck from
    /// its door instead, or waits there (rules 13).
    fn spawn_order(&self, owner: u8, kind: Kind, pos: Pos, rally: Option<Pos>) -> Order {
        if kind.gathers() {
            let rallied = rally.and_then(|rally| {
                self.nearest_salvage(owner, kind, rally, RALLY_WRECK_RADIUS, false)
            });
            match rallied {
                Some(resource) if !self.salvage_in_danger(owner, resource) => {
                    return Order::Gather { resource };
                }
                Some(_) => {
                    return match self.nearest_salvage(owner, kind, pos, AUTO_GATHER_RADIUS, true) {
                        Some(resource) => Order::Gather { resource },
                        None => Order::Idle,
                    };
                }
                None if rally.is_none() => {
                    if let Some(resource) =
                        self.nearest_salvage(owner, kind, pos, AUTO_GATHER_RADIUS, true)
                    {
                        return Order::Gather { resource };
                    }
                }
                None => {}
            }
        }
        match rally {
            Some(target) => Order::Move { target },
            None => Order::Idle,
        }
    }

    /// The nearest wreck with salvage left within `radius` cells of `from`
    /// that a machine of `kind` can gather now.  With `auto`, the pick a
    /// worker makes on its own, wrecks on tidal ground and on the sluice
    /// island are left out: those wait for an order, so a base's workers
    /// never wander into a lane on their own, and so are wrecks where
    /// `owner`'s workers were hit lately.
    fn nearest_salvage(
        &self,
        owner: u8,
        kind: Kind,
        from: Pos,
        radius: i32,
        auto: bool,
    ) -> Option<u32> {
        let limit = i64::from(FP) * i64::from(radius);
        self.map
            .resources
            .iter()
            .filter(|resource| {
                resource.kind == ResourceKind::Salvage
                    && resource.remaining > 0
                    && self.gatherable(kind, resource)
                    && (!auto
                        || (self.auto_gatherable(kind, resource)
                            && !self.in_worker_danger(owner, resource.pos)))
                    && resource.pos.distance_sq(from) <= limit.pow(2)
            })
            .min_by_key(|resource| (resource.pos.distance_sq(from), resource.id))
            .map(|resource| resource.id)
    }

    /// Whether `pos` lies in ground where `player`'s workers were hit lately.
    pub fn in_worker_danger(&self, player: u8, pos: Pos) -> bool {
        let Some(side) = self.players.get(player as usize) else {
            return false;
        };
        let reach = i64::from(FP * WORKER_DANGER_RADIUS_CELLS).pow(2);
        side.worker_danger
            .iter()
            .any(|mark| mark.until > self.tick && mark.pos.distance_sq(pos) <= reach)
    }

    fn salvage_in_danger(&self, player: u8, resource_id: u32) -> bool {
        self.map
            .resources
            .iter()
            .find(|resource| resource.id == resource_id)
            .is_some_and(|resource| self.in_worker_danger(player, resource.pos))
    }

    /// Every gathering machine and its owner, by id, before the fighting.
    fn gatherer_owners(&self) -> Vec<(u32, u8)> {
        let mut gatherers: Vec<(u32, u8)> = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && entity.kind.gathers())
            .map(|entity| (entity.id, entity.owner))
            .collect();
        gatherers.sort_unstable();
        gatherers
    }

    /// Workers under fire leave (rules 13).  A worker hit this tick marks the
    /// ground for its side; the hit worker, and every worker of its side
    /// gathering a wreck inside the mark, go to the nearest safe wreck from
    /// their drop-off, or back to the drop-off when there is none.  This
    /// happens once per hit, so a player can still order workers back in.
    fn update_worker_danger(&mut self, gatherers: &[(u32, u8)]) {
        let tick = self.tick;
        for side in &mut self.players {
            side.worker_danger.retain(|mark| mark.until > tick);
        }
        let mut hits: Vec<(u8, u32, Pos)> = self
            .events
            .iter()
            .filter(|event| event.tick == tick && event.kind == EventKind::Damage)
            .filter_map(|event| {
                let id = event.entity?;
                let index = gatherers.binary_search_by_key(&id, |(id, _)| *id).ok()?;
                let pos = self.entity(id).map(|entity| entity.pos).or(event.to)?;
                Some((gatherers[index].1, id, pos))
            })
            .collect();
        if hits.is_empty() {
            return;
        }
        hits.sort_unstable_by_key(|(owner, id, _)| (*owner, *id));
        hits.dedup_by_key(|(_, id, _)| *id);
        let until = tick.saturating_add(u64::from(WORKER_DANGER_TICKS));
        let reach = i64::from(FP * WORKER_DANGER_RADIUS_CELLS).pow(2);
        for owner in self.seats() {
            let points: Vec<Pos> = hits
                .iter()
                .filter(|(hit_owner, ..)| *hit_owner == owner)
                .map(|(_, _, pos)| *pos)
                .collect();
            if points.is_empty() {
                continue;
            }
            let marks = &mut self.players[owner as usize].worker_danger;
            for point in &points {
                if let Some(mark) = marks
                    .iter_mut()
                    .find(|mark| mark.pos.distance_sq(*point) <= i64::from(FP * 2).pow(2))
                {
                    mark.until = until;
                    continue;
                }
                if marks.len() >= MAX_DANGER_MARKS {
                    let oldest = marks
                        .iter()
                        .enumerate()
                        .min_by_key(|(index, mark)| (mark.until, *index))
                        .map(|(index, _)| index)
                        .unwrap_or(0);
                    marks.remove(oldest);
                }
                marks.push(DangerMark { pos: *point, until });
            }
            let hit_ids: Vec<u32> = hits
                .iter()
                .filter(|(hit_owner, ..)| *hit_owner == owner)
                .map(|(_, id, _)| *id)
                .collect();
            let leaving: Vec<(u32, Kind, Pos)> = self
                .entities
                .iter()
                .filter(|entity| entity.owner == owner && entity.hp > 0 && entity.kind.gathers())
                .filter_map(|entity| match entity.order {
                    Order::Gather { resource } => {
                        let wreck = self.map.resources.iter().find(|item| item.id == resource)?;
                        let threatened = hit_ids.contains(&entity.id)
                            || points
                                .iter()
                                .any(|point| point.distance_sq(wreck.pos) <= reach);
                        threatened.then_some((entity.id, entity.kind, entity.pos))
                    }
                    _ => None,
                })
                .collect();
            let mut moved = 0;
            for (id, kind, pos) in leaving {
                let current = match self.entity(id).map(|entity| &entity.order) {
                    Some(Order::Gather { resource }) => *resource,
                    _ => continue,
                };
                let dropoff = self.nearest_dropoff(owner, pos);
                let origin = dropoff.unwrap_or(pos);
                let next = self.nearest_salvage(owner, kind, origin, AUTO_GATHER_RADIUS, true);
                if next == Some(current) {
                    continue;
                }
                // With no safe wreck the worker falls back to its yard, hands
                // in its load and returns when the mark lapses (rules 22): a
                // plain move left 16-20 loaded Rakers at the Kiln in trial 12.
                let order = match (next, self.delivery_yard(owner, pos)) {
                    (Some(resource), _) => Order::Gather { resource },
                    (None, Some(target)) => Order::Deliver {
                        target,
                        resource: Some(current),
                    },
                    (None, None) => Order::Idle,
                };
                if let Some(entity) = self.entity_mut(id) {
                    entity.order = order;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.gather_ticks = 0;
                }
                moved += 1;
            }
            if moved > 0 {
                self.events.push(Event {
                    tick,
                    kind: EventKind::WorkersFled,
                    player: Some(owner),
                    entity: None,
                    other: None,
                    from: Some(points[0]),
                    to: None,
                    amount: moved,
                    text: "workers leave the wreck under fire".to_string(),
                    cause: None,
                });
            }
        }
    }

    /// Whether a worker may pick this wreck on its own: not on tidal
    /// ground, not on the sluice island; a Dredger picks anywhere.
    fn auto_gatherable(&self, kind: Kind, resource: &Resource) -> bool {
        if kind == Kind::Dredger {
            return true;
        }
        let (x, y) = resource.pos.cell_xy();
        !self.map.terrain(x, y).is_tidal() && !self.map.island_cell(x, y)
    }

    /// Artillery on an attack-move stops at its own reach and deploys there
    /// instead of walking into the enemy packed, unable to fire.  Only an
    /// attack-move does this: a plain move is the player saying where to go.
    /// The Bulwark does the same since rules 14, turned to face the nearest
    /// enemy in reach first, since its shield covers only the front: in the
    /// eighth trial seven Bulwarks walked into a Loom line packed.
    /// Whether a gun has a visible enemy inside its reach now.
    fn enemy_in_reach(&self, entity: &Entity) -> bool {
        let range = self.weapon_range(entity);
        let min_range = if entity.kind == Kind::Loom {
            LOOM_MIN_RANGE
        } else {
            0
        };
        self.entities.iter().any(|target| {
            target.owner != entity.owner
                && target.hp > 0
                && target.aboard.is_none()
                && self.entity_visible(entity.owner, target.id)
                && entity.pos.distance_sq(target.pos) <= i64::from(range).pow(2)
                && entity.pos.distance_sq(target.pos) >= i64::from(min_range).pow(2)
        })
    }

    /// A deployed gun that finished its fight takes up the attack-move it
    /// was given while an enemy was in reach (rules 17): it packs and goes.
    fn update_fight_then_pack(&mut self) {
        let ready: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.deployed
                    && entity.deploy_remaining == 0
                    && entity.hp > 0
                    && !entity.keep_deployed
                    && matches!(entity.after_pack, Some(Order::AttackMove { .. }))
                    && !self.enemy_in_reach(entity)
            })
            .map(|entity| entity.id)
            .collect();
        for id in ready {
            let siege = self
                .entity(id)
                .map(|entity| {
                    self.players[entity.owner as usize]
                        .upgrades
                        .contains(&Upgrade::Siege)
                })
                .unwrap_or(false);
            if let Some(entity) = self.entity_mut(id) {
                entity.deploy_target = false;
                entity.deploy_remaining = transition_ticks(entity.kind, siege);
                entity.order = Order::Deploy;
            }
        }
    }

    fn update_artillery_stance(&mut self) {
        let candidates: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                matches!(entity.kind, Kind::Loom | Kind::Bulwark | Kind::Heliostat)
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.aboard.is_none()
                    && !entity.deployed
                    && entity.deploy_remaining == 0
                    && (matches!(entity.order, Order::AttackMove { .. })
                        || (matches!(entity.order, Order::Attack { .. })
                            && matches!(entity.kind, Kind::Loom | Kind::Heliostat)))
            })
            .map(|entity| entity.id)
            .collect();
        for id in candidates {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            let range = self.weapon_range(&entity);
            let min_range = if entity.kind == Kind::Loom {
                LOOM_MIN_RANGE
            } else {
                0
            };
            // A gun sent at one target deploys when that one is in reach
            // (rules 19); on an attack-move, at the nearest.
            let only = match entity.order {
                Order::Attack { target } => Some(target),
                _ => None,
            };
            let nearest = self
                .entities
                .iter()
                .filter(|target| only.is_none_or(|id| target.id == id))
                .filter(|target| {
                    target.owner != entity.owner
                        && target.hp > 0
                        && target.aboard.is_none()
                        && self.entity_visible(entity.owner, target.id)
                        && entity.pos.distance_sq(target.pos) <= i64::from(range).pow(2)
                        && entity.pos.distance_sq(target.pos) >= i64::from(min_range).pow(2)
                        && self.line_of_sight(entity.pos, target.pos, Some(entity.id))
                })
                .min_by_key(|target| (entity.pos.distance_sq(target.pos), target.id))
                .map(|target| target.pos);
            let Some(aim) = nearest else {
                continue;
            };
            let siege = self.players[entity.owner as usize]
                .upgrades
                .contains(&Upgrade::Siege);
            if let Some(entity) = self.entity_mut(id) {
                if entity.kind == Kind::Bulwark {
                    entity.facing = direction_octant(aim.x - entity.pos.x, aim.y - entity.pos.y);
                }
                entity.deploy_target = true;
                entity.deploy_remaining = transition_ticks(entity.kind, siege);
                entity.order = Order::Deploy;
                entity.after_pack = None;
                entity.pace = 0;
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
                entity.waypoints.clear();
            }
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::Deploy,
                Some(entity.owner),
                id,
                "deploying at reach",
            ));
        }
    }

    fn update_deployment(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.deploy_remaining > 0)
            .map(|entity| entity.id)
            .collect();
        let lane_revision = self.gate.lane_revision;
        let layout = self.map.layout();
        for id in ids {
            let Some(entity) = self.entity_mut(id) else {
                continue;
            };
            entity.deploy_remaining = entity.deploy_remaining.saturating_sub(1);
            if entity.deploy_remaining == 0 {
                entity.deployed = entity.deploy_target;
                entity.order = if entity.deployed {
                    Order::Deploy
                } else {
                    Order::Idle
                };
                if !entity.deployed
                    && let Some(order) = entity.after_pack.take()
                {
                    // The move that packed it: taken up the moment it is free.
                    entity.order = order;
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.path_lane_revision = lane_revision;
                    entity.facing = layout.facing(entity.owner);
                }
                let owner = entity.owner;
                let deployed = entity.deployed;
                let entity_id = entity.id;
                self.events.push(Event::simple(
                    self.tick,
                    EventKind::Deploy,
                    Some(owner),
                    Some(entity_id),
                    if deployed { "deployed" } else { "packed" },
                ));
            }
        }
    }

    fn update_workers(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && entity.kind.gathers())
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            match entity.order {
                Order::Gather { resource } => self.update_worker_gather(id, resource),
                Order::Build { target } => self.update_worker_build(id, target),
                Order::Repair { target } => self.update_worker_repair(id, target),
                Order::Recycle { target } => self.update_worker_recycle(id, target),
                Order::Deliver { target, resource } => {
                    self.update_worker_deliver(id, target, resource)
                }
                _ => {}
            }
        }
    }

    fn update_worker_gather(&mut self, id: u32, resource_id: u32) {
        let Some(resource) = self
            .map
            .resources
            .iter()
            .find(|resource| resource.id == resource_id)
            .cloned()
        else {
            self.clear_order(id);
            return;
        };
        if resource.kind != ResourceKind::Salvage {
            self.clear_order(id);
            return;
        }
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        if worker.carried > 0 {
            let Some(dropoff) = self.nearest_dropoff(worker.owner, worker.pos) else {
                self.clear_order(id);
                return;
            };
            if worker.pos.distance_sq(dropoff) > i64::from(FP).pow(2) {
                self.ensure_path(id, dropoff);
                return;
            }
            self.deposit_load(id, Some(resource_id));
            return;
        }
        if resource.remaining == 0 {
            // A dry wreck sends the worker on to the next one with salvage
            // instead of parking it; a worker only idles when its side of
            // the basin is spent.
            match self.nearest_salvage(
                worker.owner,
                worker.kind,
                worker.pos,
                AUTO_GATHER_RADIUS,
                true,
            ) {
                Some(next) if next != resource_id => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Gather { resource: next };
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        entity.gather_ticks = 0;
                    }
                }
                _ => self.clear_order(id),
            }
            return;
        }
        let work_target = self.resource_work_target(&resource, id);
        if worker.pos.distance_sq(work_target) > i64::from(FP).pow(2) {
            self.ensure_path(id, work_target);
            return;
        }
        // A wreck under the tide waits for its lane to drain.
        if !self.gatherable(worker.kind, &resource) {
            return;
        }
        let mut amount = 0u32;
        let carry_capacity = self.carry_for(worker.owner, worker.kind);
        if let Some(entity) = self.entity_mut(id) {
            entity.gather_ticks = entity.gather_ticks.saturating_add(1);
            if entity.gather_ticks >= 60 {
                entity.gather_ticks = 0;
                amount = carry_capacity.min(resource.remaining);
                entity.carried = amount;
                entity.carried_kind = Some(ResourceKind::Salvage);
            }
        }
        if amount > 0 {
            let mut emptied = false;
            if let Some(item) = self
                .map
                .resources
                .iter_mut()
                .find(|item| item.id == resource_id)
            {
                item.remaining = item.remaining.saturating_sub(amount);
                emptied = item.remaining == 0;
            }
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Gather,
                player: Some(worker.owner),
                entity: Some(id),
                other: Some(resource_id),
                from: Some(resource.pos),
                to: None,
                amount: amount as i32,
                text: "salvage gathered".to_string(),
                cause: None,
            });
            if emptied {
                // The last load: say at once whether the workers find another
                // wreck in reach of their drop-off or stand idle.
                let origin = self
                    .nearest_dropoff(worker.owner, worker.pos)
                    .unwrap_or(resource.pos);
                let next = self.nearest_salvage(
                    worker.owner,
                    worker.kind,
                    origin,
                    AUTO_GATHER_RADIUS,
                    true,
                );
                self.events.push(Event {
                    tick: self.tick,
                    kind: EventKind::WreckEmptied,
                    player: Some(worker.owner),
                    entity: Some(id),
                    other: Some(resource_id),
                    from: Some(resource.pos),
                    to: None,
                    amount: 0,
                    text: if next.is_some() {
                        "workers move on".to_string()
                    } else {
                        "workers idle".to_string()
                    },
                    cause: None,
                });
            }
        }
    }

    /// A worker hands its load in: the salvage is its side's and the
    /// worker's hands are empty.
    fn deposit_load(&mut self, id: u32, resource: Option<u32>) {
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        if worker.carried > 0 {
            self.players[worker.owner as usize].salvage = self.players[worker.owner as usize]
                .salvage
                .saturating_add(worker.carried);
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Deposit,
                player: Some(worker.owner),
                entity: Some(id),
                other: resource,
                from: None,
                to: None,
                amount: i32::try_from(worker.carried).unwrap_or(i32::MAX),
                text: "salvage returned".to_string(),
                cause: None,
            });
        }
        if let Some(entity) = self.entity_mut(id) {
            entity.carried = 0;
            entity.carried_kind = None;
            entity.gather_ticks = 0;
            entity.path.clear();
            entity.path_index = 0;
            entity.path_target = None;
        }
    }

    /// A worker delivering (rules 22): it walks to its yard (or the
    /// nearest one if that fell), hands its load in, and then goes back to
    /// its wreck once no danger mark covers it, or to the nearest safe
    /// wreck from the yard. While its own wreck is marked and no other is
    /// safe it waits at the yard; with no wreck to go to it stands idle, so
    /// the idle count finds it.
    fn update_worker_deliver(&mut self, id: u32, target: u32, resource: Option<u32>) {
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        let yard = if self
            .entity(target)
            .is_some_and(|yard| delivers_at(yard, worker.owner))
        {
            target
        } else {
            match self.delivery_yard(worker.owner, worker.pos) {
                Some(next) => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Deliver {
                            target: next,
                            resource,
                        };
                    }
                    next
                }
                None => {
                    self.clear_order(id);
                    return;
                }
            }
        };
        let Some(building) = self.entity(yard).cloned() else {
            return;
        };
        let interaction = self.interaction_target(&building, worker.pos);
        if worker.pos.distance_sq(interaction) > i64::from(FP).pow(2) {
            self.ensure_path(id, interaction);
            return;
        }
        if worker.carried > 0 {
            self.deposit_load(id, resource);
        }
        let own = resource.filter(|&wreck| {
            self.map
                .resources
                .iter()
                .any(|item| item.id == wreck && item.remaining > 0)
        });
        let next = match own {
            Some(wreck) if !self.salvage_in_danger(worker.owner, wreck) => Some(wreck),
            Some(_) => {
                match self.nearest_salvage(
                    worker.owner,
                    worker.kind,
                    building.pos,
                    AUTO_GATHER_RADIUS,
                    true,
                ) {
                    Some(safe) => Some(safe),
                    // Wait at the yard for the mark on its wreck to lapse.
                    None => return,
                }
            }
            None => self.nearest_salvage(
                worker.owner,
                worker.kind,
                building.pos,
                AUTO_GATHER_RADIUS,
                true,
            ),
        };
        match next {
            Some(wreck) => {
                if let Some(entity) = self.entity_mut(id) {
                    entity.order = Order::Gather { resource: wreck };
                    entity.path.clear();
                    entity.path_index = 0;
                    entity.path_target = None;
                    entity.gather_ticks = 0;
                }
            }
            None => self.clear_order(id),
        }
    }

    /// The own finished headquarters or Salvage Yard nearest `pos`, where
    /// a worker hands in its load.
    fn delivery_yard(&self, player: u8, pos: Pos) -> Option<u32> {
        self.entities
            .iter()
            .filter(|entity| delivers_at(entity, player))
            .min_by_key(|entity| (entity.pos.distance_sq(pos), entity.id))
            .map(|entity| entity.id)
    }

    fn update_worker_build(&mut self, id: u32, target: u32) {
        let Some(target_entity) = self.entity(target).cloned() else {
            self.clear_order(id);
            self.after_site(id);
            return;
        };
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        if target_entity.build_remaining == 0 || target_entity.hp <= 0 {
            // Finished by another worker, or lost: on to the next site.
            self.clear_order(id);
            self.after_site(id);
            return;
        }
        let interaction = self.interaction_target(&target_entity, worker.pos);
        if worker.pos.distance_sq(interaction) > i64::from(FP).pow(2) {
            self.ensure_path(id, interaction);
        }
    }

    /// The own finished headquarters, Works or Salvage Yard nearest `pos`,
    /// where a worker is recycled.
    fn recycle_yard(&self, player: u8, pos: Pos) -> Option<u32> {
        self.entities
            .iter()
            .filter(|entity| recycles_at(entity, player))
            .min_by_key(|entity| (entity.pos.distance_sq(pos), entity.id))
            .map(|entity| entity.id)
    }

    /// A worker on its way to be recycled: it walks to its yard (or the
    /// next one if that fell) and, beside it, is broken up for
    /// RECYCLE_REFUND_PERCENT of its salvage, with any load it carried.
    fn update_worker_recycle(&mut self, id: u32, target: u32) {
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        let target = if self
            .entity(target)
            .is_some_and(|yard| recycles_at(yard, worker.owner))
        {
            target
        } else {
            match self.recycle_yard(worker.owner, worker.pos) {
                Some(next) => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Order::Recycle { target: next };
                    }
                    next
                }
                None => {
                    self.clear_order(id);
                    return;
                }
            }
        };
        let Some(yard) = self.entity(target).cloned() else {
            return;
        };
        let interaction = self.interaction_target(&yard, worker.pos);
        if worker.pos.distance_sq(interaction) > i64::from(FP).pow(2) {
            self.ensure_path(id, interaction);
            return;
        }
        let refund = spec(worker.kind).salvage * RECYCLE_REFUND_PERCENT / 100;
        let load = if worker
            .carried_kind
            .is_none_or(|kind| kind == ResourceKind::Salvage)
        {
            worker.carried
        } else {
            0
        };
        let side = &mut self.players[worker.owner as usize];
        side.salvage = side.salvage.saturating_add(refund).saturating_add(load);
        side.crew = side.crew.saturating_sub(spec(worker.kind).crew);
        self.entities.retain(|entity| entity.id != id);
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::Recycled,
            player: Some(worker.owner),
            entity: Some(id),
            other: Some(target),
            from: Some(worker.pos),
            to: None,
            amount: i32::try_from(refund.saturating_add(load)).unwrap_or(i32::MAX),
            text: worker.kind.name().to_string(),
            cause: None,
        });
    }

    fn update_worker_repair(&mut self, id: u32, target: u32) {
        let Some(target_entity) = self.entity(target).cloned() else {
            self.clear_order(id);
            return;
        };
        let Some(worker) = self.entity(id).cloned() else {
            return;
        };
        let interaction = self.interaction_target(&target_entity, worker.pos);
        if worker.pos.distance_sq(interaction) > i64::from(FP).pow(2) {
            self.ensure_path(id, interaction);
            return;
        }
        if target_entity.hp >= target_entity.max_hp {
            self.clear_order(id);
            return;
        }
        let mut repairers: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.owner == worker.owner
                    && entity.hp > 0
                    && entity.kind.is_worker()
                    && matches!(entity.order, Order::Repair { target: repair_target } if repair_target == target)
            })
            .map(|entity| entity.id)
            .collect();
        repairers.sort_unstable();
        if repairers
            .iter()
            .position(|repairer| *repairer == id)
            .is_some_and(|position| position >= MAX_REPAIR_WORKERS)
        {
            return;
        }
        let player = worker.owner as usize;
        if self.players[player].salvage == 0 {
            return;
        }
        let rate = if self.players[player].doctrine == Some(Doctrine::Hauling)
            && self.players[player].doctrine_tier >= 2
        {
            4u32
        } else {
            2u32
        };
        let amount = rate.min((target_entity.max_hp - target_entity.hp) as u32);
        self.players[player].salvage -= 1;
        if let Some(entity) = self.entity_mut(target) {
            entity.hp = entity.hp.saturating_add(amount as i32).min(entity.max_hp);
        }
        self.events.push(Event {
            tick: self.tick,
            kind: EventKind::Repair,
            player: Some(worker.owner),
            entity: Some(id),
            other: Some(target),
            from: Some(worker.pos),
            to: Some(target_entity.pos),
            amount: amount as i32,
            text: "repair".to_string(),
            cause: None,
        });
    }
}

impl World {
    /// Repair machines mend the nearest damaged own machine within reach,
    /// five hull a second, one salvage for every five hull.
    fn update_repair_auras(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && entity.kind.repairs() && entity.aboard.is_none())
            .map(|entity| entity.id)
            .collect();
        let period = u64::from(TICK_HZ as u32 / REPAIR_AURA_HULL_PER_SECOND as u32).max(1);
        if !self.tick.is_multiple_of(period) {
            return;
        }
        for id in ids {
            let Some(mender) = self.entity(id).cloned() else {
                continue;
            };
            let target = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.owner == mender.owner
                        && entity.id != mender.id
                        && entity.hp > 0
                        && entity.aboard.is_none()
                        && !entity.kind.is_building()
                        && entity.hp < entity.max_hp
                        && entity.pos.distance_sq(mender.pos)
                            <= i64::from(REPAIR_AURA_RADIUS).pow(2)
                })
                .min_by_key(|entity| (mender.pos.distance_sq(entity.pos), entity.id))
                .map(|entity| (entity.id, entity.pos));
            let Some((target_id, target_pos)) = target else {
                continue;
            };
            let player = mender.owner as usize;
            if self.players[player].salvage == 0 {
                continue;
            }
            // The mender's gather counter counts hull mended toward the next
            // salvage charge.
            let charge = self.entity(id).map_or(0, |entity| entity.gather_ticks) + 1;
            if charge >= REPAIR_AURA_HULL_PER_SALVAGE as u32 {
                self.players[player].salvage -= 1;
            }
            if let Some(entity) = self.entity_mut(id) {
                entity.gather_ticks = charge % REPAIR_AURA_HULL_PER_SALVAGE as u32;
            }
            if let Some(entity) = self.entity_mut(target_id) {
                entity.hp = entity.hp.saturating_add(1).min(entity.max_hp);
            }
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Repair,
                player: Some(mender.owner),
                entity: Some(id),
                other: Some(target_id),
                from: Some(mender.pos),
                to: Some(target_pos),
                amount: 1,
                text: "mended".to_string(),
                cause: None,
            });
        }
    }

    /// Whether an enemy combat machine of `player`'s stands beside the
    /// station, within two cells: a tide switch cannot be made then.
    pub fn station_contested_for(&self, player: u8) -> bool {
        let radius = i64::from(FP * 2).pow(2);
        self.entities.iter().any(|entity| {
            entity.owner != player
                && entity.hp > 0
                && entity.build_remaining == 0
                && is_combat_unit(entity.kind)
                && entity.pos.distance_sq(self.map.gate_pos) <= radius
        })
    }

    /// An enemy combat machine beside the station cancels a switch in its
    /// warning, and the price comes back (rules 13): a switch is refused up
    /// front while an enemy stands there, so a cancel is never a trap.
    fn update_switch_guard(&mut self) {
        let Some(owner) = self.gate.owner else {
            return;
        };
        // The ebb (rules 22) is nobody's switch: an enemy at the station
        // does not cancel it.
        if self.gate.warning_until.is_none() || self.gate.ebb_pending {
            return;
        }
        if self.station_contested_for(owner) {
            let price = if self.gate.flood_pending {
                FLOOD_PRESSURE
            } else {
                SWITCH_PRESSURE
            };
            let cap = self.pressure_cap_for(owner);
            let player = &mut self.players[owner as usize];
            let refund = price.min(cap.saturating_sub(player.pressure));
            player.pressure = player.pressure.saturating_add(refund);
            player.pressure_spent = player.pressure_spent.saturating_sub(u64::from(refund));
            self.gate.warning_until = None;
            self.gate.switch_target = None;
            self.gate.flood_pending = false;
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::SwitchCancelled,
                player: Some(owner),
                entity: None,
                other: None,
                from: Some(self.map.gate_pos),
                to: None,
                amount: i32::try_from(refund).unwrap_or(i32::MAX),
                text: format!("switch cancelled: enemy at the station, {refund} pressure back"),
                cause: None,
            });
        }
    }

    /// Light a firing Loom for the side its shell falls on.  One light per
    /// Loom position: a second shot refreshes it.
    fn reveal_firing_loom(&mut self, player: u8, pos: Pos) {
        let until = self.tick.saturating_add(u64::from(LOOM_FIRE_REVEAL_TICKS));
        if let Some(beacon) = self.beacons.iter_mut().find(|beacon| {
            beacon.owner == player && beacon.radius == LOOM_FIRE_REVEAL_RADIUS && beacon.pos == pos
        }) {
            beacon.until = beacon.until.max(until);
            return;
        }
        if self.beacons.len() >= MAX_ENTITIES {
            self.beacons.remove(0);
        }
        self.beacons.push(Beacon {
            owner: player,
            pos,
            until,
            radius: LOOM_FIRE_REVEAL_RADIUS,
            ray_to: None,
        });
    }

    /// Whether `player` holds a lane: the sluice is theirs, and an own gun
    /// machine or Caisson stands at either of the lane's bank mouths with no
    /// enemy one, and no enemy Defense Nest (rules 20), at either.  The
    /// station is part of the hold (rules 12): without it both sides stand
    /// on their own bank, each denying the other, and the tide can never
    /// be won.  A nest only blocks: it never makes its own side's hold.
    /// With three seats (rules 22) an enemy nest no longer blocks the lane:
    /// it slows the count instead, see `hold_slowed`.
    pub fn holds_lane(&self, player: u8, lane: usize) -> bool {
        if self.gate.owner != Some(player) {
            return false;
        }
        let mut own = false;
        for mouth in self.map.layout().arm_mouths(lane) {
            match self.mouth_presence(player, mouth) {
                MouthPresence::Blocked => return false,
                MouthPresence::Own => own = true,
                MouthPresence::Empty => {}
            }
        }
        own
    }

    /// Whether `player` holds a crossing mouth: an own gun machine or Caisson
    /// within reach of its point and no enemy one or enemy Defense Nest.
    pub fn holds_crossing(&self, player: u8, point: Pos) -> bool {
        self.mouth_presence(player, point) == MouthPresence::Own
    }

    /// Whether an enemy of `player` blocks a count at this mouth: an enemy
    /// machine that holds a mouth, or (rules 20) a finished enemy Defense
    /// Nest whose footprint's middle is inside the ring. With three seats
    /// (rules 22) the nest only slows: see `mouth_slowed_for`.
    pub fn mouth_blocked_for(&self, player: u8, point: Pos) -> bool {
        self.mouth_presence(player, point) == MouthPresence::Blocked
    }

    /// Whether an enemy Defense Nest in a mouth ring only slows a count
    /// instead of stopping it (rules 22): in a match of three seats, as it
    /// started. With three seats every lane is shared by two neighbours,
    /// and in trial 12 each one's nest on its own bank blocked every count
    /// from 18:05 to the end. Two seats keep the rules 20 block.
    pub fn nests_slow_counts(&self) -> bool {
        self.seat_count() > 2
    }

    /// Whether an enemy nest slows `player`'s count at this mouth (rules
    /// 22): a finished enemy Defense Nest in the ring and no enemy machine
    /// that holds a mouth, in a match of three seats.
    pub fn mouth_slowed_for(&self, player: u8, point: Pos) -> bool {
        let scan = self.mouth_scan(player, point);
        self.nests_slow_counts() && scan.enemy_nest && !scan.enemy_gun
    }

    /// Whether a seat's count runs at half rate (rules 22): it holds every
    /// lane it must, and an enemy Defense Nest stands in a mouth ring of
    /// one of them. The count then rises every other tick.
    pub fn hold_slowed(&self, seat: u8) -> bool {
        self.nests_slow_counts()
            && self.holds_every_lane(seat)
            && self.hold_arms(seat).into_iter().any(|arm| {
                self.map
                    .layout()
                    .arm_mouths(arm)
                    .into_iter()
                    .any(|mouth| self.mouth_scan(seat, mouth).enemy_nest)
            })
    }

    fn mouth_presence(&self, player: u8, point: Pos) -> MouthPresence {
        let scan = self.mouth_scan(player, point);
        if scan.enemy_gun || (scan.enemy_nest && !self.nests_slow_counts()) {
            MouthPresence::Blocked
        } else if scan.own {
            MouthPresence::Own
        } else {
            MouthPresence::Empty
        }
    }

    /// What stands in a mouth's ring for `player`: an own machine that
    /// holds a mouth, an enemy one, and a finished enemy Defense Nest
    /// whose footprint's middle is inside the ring.
    fn mouth_scan(&self, player: u8, point: Pos) -> MouthScan {
        let radius = i64::from(FP * CROSSING_HOLD_RADIUS_CELLS).pow(2);
        let mut scan = MouthScan::default();
        for entity in &self.entities {
            if entity.hp <= 0 || entity.build_remaining > 0 || entity.aboard.is_some() {
                continue;
            }
            if holds_mouth_unit(entity.kind) {
                if entity.pos.distance_sq(point) > radius {
                    continue;
                }
                if entity.owner == player {
                    scan.own = true;
                } else {
                    scan.enemy_gun = true;
                }
            } else if blocks_mouth_building(entity.kind)
                && entity.owner != player
                && footprint_middle(entity).distance_sq(point) <= radius
            {
                scan.enemy_nest = true;
            }
        }
        scan
    }

    fn update_tide_hold(&mut self) {
        if self.outcome.is_some() || self.hold_frozen() {
            return;
        }
        for player in self.seats() {
            // Every lane that touches the seat's land: both on the Split
            // Basin, the two either side of it on the Confluence.
            let held = self.holds_every_lane(player);
            // An enemy nest at one of its mouths halves the rate (rules 22,
            // three seats): the count rises on even ticks only.
            let stalled = held && self.tick % 2 == 1 && self.hold_slowed(player);
            let slot = &mut self.lane_hold[player as usize];
            *slot = if stalled {
                *slot
            } else if held {
                slot.saturating_add(1)
            } else {
                slot.saturating_sub(TIDE_HOLD_DRAIN_PER_TICK)
            };
        }
    }

    /// Report the first enemy combat machine seen in a region after twenty
    /// quiet seconds there, with the count now in view.
    fn update_enemy_reports(&mut self) {
        // Presentation history is not saved, so a loaded world starts empty.
        self.enemy_reports.resize(self.players.len(), Vec::new());
        for player in self.seats() {
            let mut counts: Vec<((i32, i32), u32, Pos)> = Vec::new();
            for entity in &self.entities {
                if entity.owner == player
                    || entity.hp <= 0
                    || entity.build_remaining > 0
                    || entity.aboard.is_some()
                    || !is_combat_unit(entity.kind)
                    || !self.entity_visible(player, entity.id)
                {
                    continue;
                }
                let (x, y) = entity.pos.cell_xy();
                let region = (
                    x.div_euclid(REPORT_REGION_CELLS),
                    y.div_euclid(REPORT_REGION_CELLS),
                );
                if let Some(slot) = counts.iter_mut().find(|slot| slot.0 == region) {
                    slot.1 += 1;
                } else {
                    counts.push((region, 1, entity.pos));
                }
            }
            let own_regions: Vec<(i32, i32)> = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.owner == player && entity.hp > 0 && entity.kind.is_building()
                })
                .map(|entity| {
                    let (x, y) = entity.pos.cell_xy();
                    (
                        x.div_euclid(REPORT_REGION_CELLS),
                        y.div_euclid(REPORT_REGION_CELLS),
                    )
                })
                .collect();
            let reports = &mut self.enemy_reports[player as usize];
            reports.retain(|(_, _, tick)| self.tick.saturating_sub(*tick) < REPORT_QUIET_TICKS * 4);
            for (region, count, pos) in counts {
                let quiet = reports
                    .iter()
                    .find(|(x, y, _)| (*x, *y) == region)
                    .is_none_or(|(_, _, tick)| {
                        self.tick.saturating_sub(*tick) >= REPORT_QUIET_TICKS
                    });
                if let Some(slot) = reports.iter_mut().find(|(x, y, _)| (*x, *y) == region) {
                    slot.2 = self.tick;
                } else {
                    reports.push((region.0, region.1, self.tick));
                }
                if quiet {
                    let at_base = own_regions.contains(&region);
                    self.events.push(Event {
                        tick: self.tick,
                        kind: EventKind::EnemySeen,
                        player: Some(player),
                        entity: None,
                        other: at_base.then_some(1),
                        from: None,
                        to: Some(pos),
                        amount: count as i32,
                        text: if at_base {
                            "enemy at base"
                        } else {
                            "enemy seen"
                        }
                        .to_string(),
                        cause: None,
                    });
                }
            }
        }
    }

    /// Damage with the keel rule: a headquarters keeps a quarter of its hull
    /// while a finished Works of its owner stands.  Returns whether the
    /// target died.
    fn deal_damage(&mut self, target_id: u32, amount: i32) -> bool {
        let Some(target) = self.entity(target_id).cloned() else {
            return false;
        };
        let keel = target.kind == Kind::Headquarters
            && self.entities.iter().any(|entity| {
                entity.owner == target.owner
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.kind == Kind::Works
            });
        let floor = if keel {
            target.max_hp.saturating_mul(KEEL_PERCENT) / 100
        } else {
            0
        };
        let Some(entity) = self.entity_mut(target_id) else {
            return false;
        };
        let next = entity.hp.saturating_sub(amount);
        entity.hp = if keel && entity.hp > floor {
            next.max(floor)
        } else {
            next
        };
        entity.hp <= 0
    }

    /// A destroyed combat machine leaves a wreck worth half its salvage
    /// cost, or all of it when the killer's side has Scrap Recovery: a won
    /// fight leaves salvage on the ground for whoever holds it.
    fn leave_scrap(&mut self, fallen: &Entity, killer: Option<u8>) {
        let Some(killer) = killer else {
            return;
        };
        let percent = if self.players[killer as usize]
            .upgrades
            .contains(&Upgrade::ScrapRecovery)
        {
            SCRAP_UPGRADED_PERCENT
        } else {
            SCRAP_BASE_PERCENT
        };
        self.leave_scrap_at(fallen, percent);
    }

    /// A wreck worth `percent` of a fallen combat machine's salvage cost,
    /// less at a crossing mouth.
    fn leave_scrap_at(&mut self, fallen: &Entity, percent: u32) {
        if !is_combat_unit(fallen.kind) || self.map.resources.len() >= MAX_RESOURCES {
            return;
        }
        let (x, y) = fallen.pos.cell_xy();
        if !self.map.terrain(x, y).walkable() {
            return;
        }
        // A machine that falls at a crossing mouth leaves less: the mouth is
        // where holds are fought, and a whole failed push there once paid
        // the defender enough to decide the match (rules 15).
        let reach = i64::from(MOUTH_SCRAP_RADIUS_CELLS * FP).pow(2);
        let percent = if self
            .map
            .layout()
            .mouths()
            .flatten()
            .any(|mouth| fallen.pos.distance_sq(mouth) <= reach)
        {
            percent.saturating_mul(MOUTH_SCRAP_PERCENT) / 100
        } else {
            percent
        };
        let id = self
            .map
            .resources
            .iter()
            .map(|resource| resource.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.map.resources.push(Resource {
            scrap: true,
            id,
            pos: Pos::cell(x, y),
            remaining: spec(fallen.kind).salvage.saturating_mul(percent) / 100,
            kind: ResourceKind::Salvage,
        });
    }
}

fn direction_octant(dx: i32, dy: i32) -> u8 {
    if dx == 0 && dy == 0 {
        return 0;
    }
    let ax = i64::from(dx).unsigned_abs();
    let ay = i64::from(dy).unsigned_abs();
    // Quantize to the nearest of eight compass directions with an integer
    // 22.5-degree boundary (tan(22.5 degrees) ~= 0.414).  This avoids
    // subcell jitter turning an intended cardinal Face into a diagonal.
    let diagonal = ay.saturating_mul(1000) > ax.saturating_mul(414)
        && ax.saturating_mul(1000) > ay.saturating_mul(414);
    if !diagonal {
        if ax >= ay {
            return if dx > 0 { 2 } else { 6 };
        }
        return if dy > 0 { 4 } else { 0 };
    }
    match (dx.signum(), dy.signum()) {
        (1, -1) => 1,
        (1, 1) => 3,
        (-1, 1) => 5,
        (-1, -1) => 7,
        _ => 0,
    }
}

/// The line machines Plate armours: the Bulwark and Reedguard, and since
/// rules 17 the Riveter, the Union's core line.  In the ninth trial Plate
/// covered every Assembly Reedguard but only Union's twelve Bulwarks, and
/// Reedguards killed 47 of Union's 74 machines.  Public so the upgrade's
/// tooltip names exactly these machines.
pub fn plated(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Bulwark | Kind::Reedguard | Kind::Riveter | Kind::Brander | Kind::Heliostat
    )
}

/// Ticks to deploy or pack: a second, half that for a Loom or Heliostat
/// with Siege.
fn transition_ticks(kind: Kind, siege: bool) -> u32 {
    if matches!(kind, Kind::Loom | Kind::Heliostat) && siege {
        TICK_HZ as u32 / 2
    } else {
        TICK_HZ as u32
    }
}

/// A machine with nothing to do: idle, holding, or at the end of a move
/// (a new machine walks a few cells from its door since rules 14).
fn at_rest(entity: &Entity) -> bool {
    match entity.order {
        Order::Idle | Order::Hold => true,
        Order::Move { .. } => entity.path.is_empty() && entity.waypoints.is_empty(),
        _ => false,
    }
}

/// Whether `cell` lies under a building's footprint.
fn footprint_overlaps_site(building: &Entity, cell: (i32, i32)) -> bool {
    let (x, y) = building.pos.cell_xy();
    let size = spec(building.kind).footprint.max(1);
    cell.0 >= x && cell.0 < x + size && cell.1 >= y && cell.1 < y + size
}

pub fn is_specialist(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Bulwark | Kind::Loom | Kind::Caisson | Kind::Heliostat | Kind::Pan
    )
}

fn is_combat_unit(kind: Kind) -> bool {
    !kind.is_worker() && !kind.is_building() && spec(kind).damage > 0
}

/// Where a worker is recycled: an own finished headquarters, Works or
/// Salvage Yard.
/// Whether a worker of `player` hands its load in at this building: a
/// finished own headquarters or Salvage Yard, the drop-offs.
pub fn delivers_at(entity: &Entity, player: u8) -> bool {
    entity.owner == player
        && entity.hp > 0
        && entity.build_remaining == 0
        && matches!(entity.kind, Kind::Headquarters | Kind::Dropoff)
}

fn recycles_at(entity: &Entity, player: u8) -> bool {
    entity.owner == player
        && entity.hp > 0
        && entity.build_remaining == 0
        && matches!(
            entity.kind,
            Kind::Headquarters | Kind::Works | Kind::Dropoff
        )
}

/// A hunter's extra damage a shot (rules 20-22): the Sounder against a
/// deployed Loom or Heliostat, the Glinter against a deployed Loom.
pub fn hunter_bonus(attacker: &Entity, target: &Entity) -> i32 {
    if !target.deployed {
        return 0;
    }
    match (attacker.kind, target.kind) {
        (Kind::Sounder, Kind::Loom | Kind::Heliostat) => SOUNDER_LOOM_BONUS,
        (Kind::Glinter, Kind::Loom) => GLINTER_LOOM_BONUS,
        _ => 0,
    }
}

/// What counts at a crossing mouth: every gun machine, and the Caisson,
/// whose whole role is to hold one.  Only these make a hold.
fn holds_mouth_unit(kind: Kind) -> bool {
    is_combat_unit(kind) || kind == Kind::Caisson
}

/// How long a count must run to win (rules 20), for a match of `seats`
/// seats, once any seat is out or not: 90 s with two seats, 75 s while
/// three or more stand, 120 s after a seat is out.
pub fn hold_ticks_for(seats: usize, any_out: bool) -> u32 {
    if seats <= 2 {
        TIDE_HOLD_TICKS
    } else if any_out {
        TIDE_HOLD_AFTER_OUT_TICKS
    } else {
        TIDE_HOLD_THREE_SEAT_TICKS
    }
}

/// A building that blocks an enemy's count at a mouth without holding one
/// (rules 20): the Defense Nest.
pub fn blocks_mouth_building(kind: Kind) -> bool {
    kind == Kind::Tower
}

/// The middle of a building's footprint, as the placement ghost measures
/// it.
pub fn footprint_middle(entity: &Entity) -> Pos {
    let n = spec(entity.kind).footprint.max(1);
    let (x, y) = entity.pos.cell_xy();
    let origin = Pos::cell(x, y);
    Pos {
        x: origin.x + (n - 1) * FP / 2,
        y: origin.y + (n - 1) * FP / 2,
    }
}

/// What stands in a mouth's ring for one seat.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MouthPresence {
    Empty,
    Own,
    Blocked,
}

/// Everything in a mouth's ring that matters to one seat's count.
#[derive(Clone, Copy, Default)]
struct MouthScan {
    own: bool,
    enemy_gun: bool,
    enemy_nest: bool,
}

/// The unit step of a facing octant: 0 is north, 2 east, 4 south, 6 west.
fn octant_vector(direction: u8) -> (i32, i32) {
    match direction & 7 {
        0 => (0, -1),
        1 => (1, -1),
        2 => (1, 0),
        3 => (1, 1),
        4 => (0, 1),
        5 => (-1, 1),
        6 => (-1, 0),
        _ => (-1, -1),
    }
}

fn in_front_arc(entity: &Entity, attacker_pos: Pos) -> bool {
    let (forward_x, forward_y) = octant_vector(entity.facing);
    let (forward_x, forward_y) = (i64::from(forward_x), i64::from(forward_y));
    let dx = i64::from(attacker_pos.x) - i64::from(entity.pos.x);
    let dy = i64::from(attacker_pos.y) - i64::from(entity.pos.y);
    dx.saturating_mul(forward_x)
        .saturating_add(dy.saturating_mul(forward_y))
        >= 0
}

impl Event {
    fn simple(
        tick: u64,
        kind: EventKind,
        player: Option<u8>,
        entity: Option<u32>,
        text: &str,
    ) -> Self {
        Self {
            tick,
            kind,
            player,
            entity,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: text.to_string(),
            cause: None,
        }
    }

    fn text(tick: u64, kind: EventKind, player: Option<u8>, text: impl Into<String>) -> Self {
        Self {
            tick,
            kind,
            player,
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: text.into(),
            cause: None,
        }
    }

    fn with_entity(
        tick: u64,
        kind: EventKind,
        player: Option<u8>,
        entity: u32,
        text: impl Into<String>,
    ) -> Self {
        Self {
            tick,
            kind,
            player,
            entity: Some(entity),
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: text.into(),
            cause: None,
        }
    }
}

impl World {
    fn update_combat(&mut self) {
        for entity in &mut self.entities {
            entity.attack_cooldown = entity.attack_cooldown.saturating_sub(1);
        }
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.deploy_remaining == 0
                    && entity.aboard.is_none()
            })
            .map(|entity| entity.id)
            .collect();
        let mut damage_events = Vec::new();
        for id in ids {
            let Some(attacker) = self.entity(id).cloned() else {
                continue;
            };
            let target_id = match attacker.order {
                Order::Attack { target } => {
                    if self.entity_visible(attacker.owner, target) {
                        Some(target)
                    } else {
                        None
                    }
                }
                // An idle machine or defence nest fires at what comes into
                // its weapon range without moving; pursuit is decided in
                // `update_idle_engagement`.
                Order::AttackMove { .. } | Order::Hold | Order::Deploy | Order::Idle => {
                    self.acquire_target(&attacker)
                }
                _ => None,
            };
            let Some(target_id) = target_id else {
                continue;
            };
            let Some(target) = self.entity(target_id).cloned() else {
                continue;
            };
            if target.owner == attacker.owner || target.hp <= 0 {
                continue;
            }
            let range = self.weapon_range(&attacker);
            if attacker.kind == Kind::Loom
                && (!attacker.deployed
                    || attacker.pos.distance_sq(target.pos) < i64::from(LOOM_MIN_RANGE).pow(2))
            {
                continue;
            }
            if attacker.kind == Kind::Heliostat && !attacker.deployed {
                continue;
            }
            if attacker.kind == Kind::Bulwark
                && attacker.deployed
                && !in_front_arc(&attacker, target.pos)
            {
                // A deployed Bulwark is committed to its facing arc.  It
                // cannot rotate as a side effect of acquiring a target.
                continue;
            }
            if attacker.pos.distance_sq(target.pos) > i64::from(range).pow(2)
                || !self.line_of_sight(attacker.pos, target.pos, Some(attacker.id))
            {
                continue;
            }
            if attacker.surge_remaining > 0
                || attacker.attack_cooldown > 0
                || spec(attacker.kind).damage <= 0
            {
                continue;
            }
            if attacker.kind == Kind::Loom {
                if self.artillery.len() >= MAX_ARTILLERY_SHOTS {
                    continue;
                }
                let impact_tick = self.tick.saturating_add(u64::from(LOOM_WINDUP_TICKS));
                self.artillery.push(ArtilleryShot {
                    owner: attacker.owner,
                    source: attacker.id,
                    from: attacker.pos,
                    target: target.pos,
                    impact_tick,
                });
                let cooldown = self.attack_cooldown(&attacker);
                if let Some(entity) = self.entity_mut(id) {
                    entity.attack_cooldown = cooldown;
                }
                self.reveal_firing_loom(target.owner, attacker.pos);
                self.events.push(Event {
                    tick: self.tick,
                    kind: EventKind::ArtilleryWarning,
                    player: Some(attacker.owner),
                    entity: Some(attacker.id),
                    other: Some(target.id),
                    from: Some(attacker.pos),
                    to: Some(target.pos),
                    amount: spec(Kind::Loom).damage,
                    text: "Loom shot committed".to_string(),
                    cause: None,
                });
                continue;
            }
            let mut base = spec(attacker.kind).damage;
            if attacker.kind == Kind::Heliostat {
                // The beam builds on one target and is seen where it
                // comes from, like a Loom's shell.
                base = self.beam_damage(&attacker, target_id);
                let tick = self.tick;
                if let Some(entity) = self.entity_mut(id) {
                    entity.beam = Some(Beam {
                        target: target_id,
                        damage: base,
                        last_tick: tick,
                    });
                }
                self.reveal_firing_loom(target.owner, attacker.pos);
            }
            damage_events.push(DamageEvent {
                attacker: id,
                target: target_id,
                amount: self.tempered(attacker.owner, attacker.kind, base),
                from: attacker.pos,
                to: target.pos,
            });
        }
        damage_events.sort_by_key(|event| (event.target, event.attacker));
        let mut dead = Vec::new();
        let mut killers: Vec<(u32, u8, u32, Kind)> = Vec::new();
        for event in damage_events {
            let attacker_cooldown = self
                .entity(event.attacker)
                .map(|attacker| self.attack_cooldown(attacker))
                .unwrap_or_default();
            if let Some(attacker) = self.entity_mut(event.attacker) {
                attacker.attack_cooldown = attacker_cooldown;
                if !(attacker.kind == Kind::Bulwark && attacker.deployed) {
                    attacker.facing =
                        direction_octant(event.to.x - event.from.x, event.to.y - event.from.y);
                }
            }
            let attacker = self
                .entity(event.attacker)
                .map(|entity| (entity.owner, entity.kind));
            let attacker_owner = attacker.map(|(owner, _)| owner);
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Shot,
                player: attacker_owner,
                entity: Some(event.attacker),
                other: Some(event.target),
                from: Some(event.from),
                to: Some(event.to),
                amount: event.amount,
                text: "shot".to_string(),
                cause: None,
            });
            let amount = self.modified_damage(event.attacker, event.target, event.amount);
            if self.deal_damage(event.target, amount) {
                dead.push(event.target);
                if let Some((owner, kind)) = attacker {
                    killers.push((event.target, owner, event.attacker, kind));
                }
            }
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Damage,
                player: attacker_owner,
                entity: Some(event.target),
                other: Some(event.attacker),
                from: Some(event.from),
                to: Some(event.to),
                amount,
                text: "damage".to_string(),
                cause: None,
            });
        }
        dead.sort_unstable();
        dead.dedup();
        for id in dead {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            let killer = killers.iter().find(|(target, ..)| *target == id).copied();
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Death,
                player: Some(entity.owner),
                entity: Some(entity.id),
                other: killer.map(|(_, _, attacker, _)| attacker),
                from: Some(entity.pos),
                to: None,
                amount: 0,
                text: entity.kind.name().to_string(),
                cause: killer.map(|(_, _, _, kind)| kind),
            });
            self.players[entity.owner as usize].crew = self.players[entity.owner as usize]
                .crew
                .saturating_sub(spec(entity.kind).crew);
            if entity.kind == Kind::Headquarters {
                self.players[entity.owner as usize].research = None;
            }
            self.leave_scrap(&entity, killer.map(|(_, owner, ..)| owner));
        }
        self.sink_cargo();
        self.entities.retain(|entity| entity.hp > 0);
    }

    /// The damage of a Heliostat's next shot at `target`: one step over the
    /// last on the same target, or a fresh start.
    fn beam_damage(&self, heliostat: &Entity, target: u32) -> i32 {
        let first = spec(Kind::Heliostat).damage;
        let Some(beam) = heliostat.beam else {
            return first;
        };
        let fresh = self.tick.saturating_sub(beam.last_tick)
            > u64::from(bw_content::HELIOSTAT_BEAM_RESET_TICKS);
        if beam.target != target || fresh {
            return first;
        }
        let step = if self.players[heliostat.owner as usize]
            .upgrades
            .contains(&Upgrade::Siege)
        {
            2 * bw_content::HELIOSTAT_BEAM_STEP
        } else {
            bw_content::HELIOSTAT_BEAM_STEP
        };
        beam.damage
            .saturating_add(step)
            .min(bw_content::HELIOSTAT_BEAM_MAX)
    }

    fn modified_damage(&self, attacker: u32, target: u32, amount: i32) -> i32 {
        let Some(attacker_entity) = self.entity(attacker) else {
            return amount;
        };
        let Some(target_entity) = self.entity(target) else {
            return amount;
        };
        if target_entity.kind == Kind::Bulwark
            && target_entity.deployed
            && in_front_arc(target_entity, attacker_entity.pos)
        {
            return (amount / 2).max(1);
        }
        // The hunters (rules 20-22): the Union's Sounder against a deployed
        // Loom or Heliostat, the Compact's Glinter against a deployed Loom.
        // Since rules 22 the bonus is part of the shot, so a wading or
        // swamped target takes the water share of it too.
        let mut amount = amount.saturating_add(hunter_bonus(attacker_entity, target_entity));
        if target_entity.kind.is_building() && attacker_entity.kind == Kind::Riveter {
            let percent = if self.players[attacker_entity.owner as usize]
                .upgrades
                .contains(&Upgrade::Siege)
            {
                200
            } else {
                RIVETER_BUILDING_DAMAGE_PERCENT
            };
            amount = amount.saturating_mul(percent).saturating_div(100);
        }
        if target_entity.kind.is_building() && attacker_entity.kind == Kind::Heliostat {
            amount = amount
                .saturating_mul(bw_content::HELIOSTAT_BUILDING_DAMAGE_PERCENT)
                .saturating_div(100);
        }
        if self.swamped(target_entity) {
            amount = amount
                .saturating_mul(SWAMPED_DAMAGE_PERCENT)
                .saturating_div(100);
        } else if self.wading(target_entity) {
            amount = amount
                .saturating_mul(WADING_DAMAGE_PERCENT)
                .saturating_div(100);
        }
        amount
    }

    /// A machine standing in shallow tidal water.
    pub fn wading(&self, entity: &Entity) -> bool {
        let (x, y) = entity.pos.cell_xy();
        !matches!(spec(entity.kind).movement, Movement::Hull | Movement::Air)
            && self.depth_at(x, y) == Some(Depth::Shallow)
    }

    /// A ground machine caught in deep tidal water.
    pub fn swamped(&self, entity: &Entity) -> bool {
        let (x, y) = entity.pos.cell_xy();
        !matches!(spec(entity.kind).movement, Movement::Hull | Movement::Air)
            && entity.kind != Kind::Dredger
            && self.depth_at(x, y) == Some(Depth::Deep)
    }

    fn update_artillery(&mut self) {
        if self.artillery.is_empty() {
            return;
        }
        let mut ready = Vec::new();
        let mut pending = Vec::with_capacity(self.artillery.len());
        for shot in self.artillery.drain(..) {
            if shot.impact_tick <= self.tick {
                ready.push(shot);
            } else {
                pending.push(shot);
            }
        }
        self.artillery = pending;
        ready.sort_by_key(|shot| {
            (
                shot.impact_tick,
                shot.owner,
                shot.source,
                shot.target.y,
                shot.target.x,
            )
        });
        let blast_radius_sq = i64::from(LOOM_BLAST_RADIUS).pow(2);
        let mut dead = Vec::new();
        for shot in ready {
            let mut victims: Vec<u32> = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.hp > 0
                        && entity.owner != shot.owner
                        && entity.pos.distance_sq(shot.target) <= blast_radius_sq
                })
                .map(|entity| entity.id)
                .collect();
            victims.sort_unstable();
            // Keep one shell event per arriving projectile while retaining a
            // deterministic representative victim for presentation metadata.
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Shot,
                player: Some(shot.owner),
                entity: Some(shot.source),
                other: victims.first().copied(),
                from: Some(shot.from),
                to: Some(shot.target),
                amount: spec(Kind::Loom).damage,
                text: "Loom blast".to_string(),
                cause: None,
            });
            for target_id in victims {
                let Some(target) = self.entity(target_id).cloned() else {
                    continue;
                };
                if target.hp <= 0
                    || target.owner == shot.owner
                    || target.pos.distance_sq(shot.target) > blast_radius_sq
                {
                    continue;
                }
                let amount = self.artillery_damage(&shot, &target);
                if self.deal_damage(target_id, amount) {
                    dead.push((target_id, shot.owner, shot.source));
                }
                self.events.push(Event {
                    tick: self.tick,
                    kind: EventKind::Damage,
                    player: Some(shot.owner),
                    entity: Some(target_id),
                    other: Some(shot.source),
                    from: Some(shot.from),
                    to: Some(shot.target),
                    amount,
                    text: "Loom blast damage".to_string(),
                    cause: None,
                });
            }
        }
        dead.sort_unstable();
        dead.dedup_by_key(|(id, ..)| *id);
        for (id, killer, source) in dead {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Death,
                player: Some(entity.owner),
                entity: Some(entity.id),
                other: Some(source),
                from: Some(entity.pos),
                to: None,
                amount: 0,
                text: entity.kind.name().to_string(),
                cause: Some(Kind::Loom),
            });
            self.players[entity.owner as usize].crew = self.players[entity.owner as usize]
                .crew
                .saturating_sub(spec(entity.kind).crew);
            if entity.kind == Kind::Headquarters {
                self.players[entity.owner as usize].research = None;
            }
            self.leave_scrap(&entity, Some(killer));
        }
        self.sink_cargo();
        self.entities.retain(|entity| entity.hp > 0);
    }

    fn artillery_damage(&self, shot: &ArtilleryShot, target: &Entity) -> i32 {
        let amount = self.tempered(shot.owner, Kind::Loom, spec(Kind::Loom).damage);
        // The hunters are built to close on a Loom line (rules 22): a shell
        // does them half its damage.
        let amount = if matches!(target.kind, Kind::Sounder | Kind::Glinter) {
            amount
                .saturating_mul(HUNTER_LOOM_SHELL_PERCENT)
                .saturating_div(100)
                .max(1)
        } else {
            amount
        };
        let amount =
            if target.kind == Kind::Bulwark && target.deployed && in_front_arc(target, shot.from) {
                (amount / 2).max(1)
            } else if target.kind.is_building() {
                amount
                    .saturating_mul(LOOM_BUILDING_DAMAGE_PERCENT)
                    .saturating_div(100)
            } else {
                amount
            };
        if self.wading(target) {
            amount
                .saturating_mul(WADING_DAMAGE_PERCENT)
                .saturating_div(100)
        } else {
            amount
        }
    }

    /// Idle combat machines engage on their own: a visible enemy inside the
    /// machine's sight but outside its weapon range draws an attack-move to
    /// where that enemy stands.  The machine fires on the way and stops where
    /// the enemy was, so the pursuit is bounded and the machine returns to
    /// idle when nothing is in sight.  Workers, buildings, deployed
    /// specialists and Looms (which must deploy to fire) do not pursue.
    fn update_idle_engagement(&mut self) {
        let candidates: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.deploy_remaining == 0
                    && !entity.deployed
                    && !entity.kind.is_building()
                    && !entity.kind.is_worker()
                    && entity.kind != Kind::Loom
                    && entity.surge_remaining == 0
                    && matches!(entity.order, Order::Idle)
                    && spec(entity.kind).damage > 0
            })
            .map(|entity| entity.id)
            .collect();
        for id in candidates {
            let Some(machine) = self.entity(id).cloned() else {
                continue;
            };
            let sight = i64::from(self.sight_of(&machine));
            let range = i64::from(self.weapon_range(&machine));
            let seen = self
                .entities
                .iter()
                .filter(|enemy| {
                    enemy.owner != machine.owner
                        && enemy.hp > 0
                        && enemy.build_remaining == 0
                        && self.entity_visible(machine.owner, enemy.id)
                        && machine.pos.distance_sq(enemy.pos) <= sight.pow(2)
                        && self.line_of_sight(machine.pos, enemy.pos, Some(machine.id))
                })
                // Machines before buildings, then the nearest, then the lowest id.
                .min_by_key(|enemy| {
                    (
                        enemy.kind.is_building(),
                        machine.pos.distance_sq(enemy.pos),
                        enemy.id,
                    )
                })
                .cloned();
            let Some(enemy) = seen else {
                continue;
            };
            if machine.pos.distance_sq(enemy.pos) <= range.pow(2) {
                // In weapon range already: the combat pass fires from idle.
                continue;
            }
            if let Some(entity) = self.entity_mut(id) {
                entity.order = Order::AttackMove { target: enemy.pos };
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
            }
        }
    }

    fn acquire_target(&self, attacker: &Entity) -> Option<u32> {
        self.entities
            .iter()
            .filter(|entity| {
                entity.owner != attacker.owner
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.aboard.is_none()
                    && self.entity_visible(attacker.owner, entity.id)
                    && self.line_of_sight(attacker.pos, entity.pos, Some(attacker.id))
                    && attacker.pos.distance_sq(entity.pos)
                        <= i64::from(self.weapon_range(attacker)).pow(2)
                    && (!is_specialist(attacker.kind)
                        || (attacker.kind == Kind::Loom
                            && attacker.deployed
                            && attacker.pos.distance_sq(entity.pos)
                                >= i64::from(LOOM_MIN_RANGE).pow(2))
                        || (attacker.kind == Kind::Bulwark
                            && (!attacker.deployed || in_front_arc(attacker, entity.pos)))
                        || (attacker.kind == Kind::Heliostat && attacker.deployed))
            })
            .min_by_key(|entity| (attacker.pos.distance_sq(entity.pos), entity.id))
            .map(|entity| entity.id)
    }

    fn update_gate_capture(&mut self) {
        let radius = i64::from(FP * 2).pow(2);
        let mut candidates = Vec::new();
        for player in self.seats() {
            let mut channelers: Vec<u32> = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.owner == player
                        && entity.hp > 0
                        && entity.build_remaining == 0
                        && !entity.kind.is_worker()
                        && !entity.kind.is_building()
                        && entity.surge_remaining == 0
                        && matches!(entity.order, Order::Capture)
                        && entity.pos.distance_sq(self.map.gate_pos) <= radius
                })
                .map(|entity| entity.id)
                .collect();
            channelers.sort_unstable();
            if !channelers.is_empty() {
                candidates.push((player, channelers));
            }
        }
        if candidates.len() == 1 {
            let player = candidates[0].0;
            let channelers = candidates[0].1.clone();
            let channeler = channelers[0];
            let work = (channelers.len() as u32).min(CAPTURE_MAX_WORKERS);
            let contest = i64::from(FP * CAPTURE_CONTEST_RADIUS_CELLS).pow(2);
            let enemy_present = self.entities.iter().any(|entity| {
                entity.owner != player
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.aboard.is_none()
                    && !entity.kind.is_worker()
                    && !entity.kind.is_building()
                    && entity.pos.distance_sq(self.map.gate_pos) <= contest
            });
            if enemy_present {
                return;
            }
            if self.gate.owner == Some(player) {
                self.gate.capture_player = None;
                self.gate.capture_progress = 0;
                self.gate.capture_missing_ticks = 0;
                for id in &channelers {
                    self.clear_order(*id);
                }
                let _ = channeler;
                return;
            }
            self.gate.capture_missing_ticks = 0;
            if self.gate.capture_player != Some(player) {
                self.gate.capture_player = Some(player);
                self.gate.capture_progress = 0;
                self.events.push(Event::text(
                    self.tick,
                    EventKind::GateCaptureStarted,
                    Some(player),
                    "capture started",
                ));
            }
            self.gate.capture_progress = self
                .gate
                .capture_progress
                .saturating_add(work)
                .min(self.capture_work());
            if self.gate.capture_progress >= self.capture_work() {
                let changed_owner = self.gate.owner != Some(player);
                self.gate.owner = Some(player);
                self.gate.capture_player = None;
                self.gate.capture_progress = 0;
                for id in &channelers {
                    self.clear_order(*id);
                }
                if changed_owner {
                    self.events.push(Event::text(
                        self.tick,
                        EventKind::GateCaptured,
                        Some(player),
                        "gate captured",
                    ));
                }
            }
        } else if candidates.is_empty() && self.gate.capture_progress > 0 {
            self.gate.capture_missing_ticks = self.gate.capture_missing_ticks.saturating_add(1);
            if self.gate.capture_missing_ticks >= 2 * TICK_HZ as u32 {
                self.gate.capture_progress = 0;
                self.gate.capture_player = None;
                self.gate.capture_missing_ticks = 0;
            }
        }
    }

    /// Whether a DRY ebbs back to the neutral tide on this map (rules 22):
    /// on a map of three arms, where a DRY leaves two arms deep and the seat
    /// between them an island. The Split Basin keeps a DRY until the next
    /// switch.
    pub fn dry_ebbs(&self) -> bool {
        self.map.layout().arm_count() > 2
    }

    /// Start the ebb's warning when it is due (rules 22): the usual
    /// switch warning, so the tide is seen and heard coming. It waits while
    /// another warning runs or a flood stands; a DRY that lands meanwhile
    /// sets a new ebb.
    fn schedule_ebb(&mut self) {
        let Some(at) = self.gate.ebb_at else {
            return;
        };
        if self.gate.tide != Tide::Open || self.gate.warning_until.is_some() {
            return;
        }
        if self.tick < at.saturating_sub(GATE_WARNING_TICKS as u64) {
            return;
        }
        self.gate.warning_until = Some(self.tick.saturating_add(GATE_WARNING_TICKS as u64));
        self.gate.ebb_pending = true;
        self.gate.switch_target = None;
        self.gate.flood_pending = false;
        self.events.push(Event::text(
            self.tick,
            EventKind::GateWarning,
            self.gate.owner,
            "the dry arm ebbs",
        ));
    }

    fn apply_gate_schedule(&mut self) {
        self.schedule_ebb();
        if let Some(warning_until) = self.gate.warning_until
            && self.tick >= warning_until
        {
            let text = if self.gate.flood_pending {
                self.gate.flood_pending = false;
                self.gate.tide = Tide::Flood;
                self.gate.flood_until = Some(self.tick.saturating_add(u64::from(FLOOD_TICKS)));
                "flood"
            } else if self.gate.ebb_pending {
                // Every arm shallow again, as at the start: a later flood
                // falls back to this neutral tide.
                self.gate.ebb_pending = false;
                self.gate.ebb_at = None;
                self.gate.tide = Tide::Neutral;
                self.gate.opened = false;
                "tide ebbs"
            } else {
                let target = self.gate.switch_target.take().unwrap_or(self.next_arm());
                self.gate.dry_arm = target;
                self.gate.tide = Tide::Open;
                self.gate.opened = true;
                if self.dry_ebbs() {
                    self.gate.ebb_at = Some(self.tick.saturating_add(u64::from(DRY_EBB_TICKS)));
                }
                self.map
                    .layout()
                    .arms
                    .get(target.index())
                    .map_or("open", |arm| arm.opened)
            };
            self.gate.switch_target = None;
            self.gate.warning_until = None;
            self.gate.transition_until = None;
            self.gate.locked_until = self.tick.saturating_add(GATE_LOCK_TICKS as u64);
            self.tide_changed(text);
        }
        if let Some(until) = self.gate.flood_until
            && self.tick >= until
        {
            // The flood falls back to the open side, or to the neutral tide
            // if no side was ever opened. A DRY whose ebb came due under
            // the flood (rules 22) is gone: the flood falls to neutral.
            self.gate.flood_until = None;
            if self.gate.ebb_at.is_some_and(|at| self.tick >= at) {
                self.gate.ebb_at = None;
                self.gate.opened = false;
            }
            self.gate.tide = if self.gate.opened {
                Tide::Open
            } else {
                Tide::Neutral
            };
            self.gate.locked_until = self.tick.saturating_add(GATE_LOCK_TICKS as u64);
            // The falling flood leaves flotsam on the four lane wrecks
            // (rules 15): FLOOD refills the wrecks at the crossing mouths,
            // and the lane that dries after it opens them to the holder.
            let layout = self.map.layout();
            for resource in &mut self.map.resources {
                if !resource.scrap && layout.is_lane_wreck(resource.pos.cell_xy()) {
                    resource.remaining = resource
                        .remaining
                        .saturating_add(FLOOD_FLOTSAM)
                        .min(LANE_WRECK);
                }
            }
            self.tide_changed("tide falls");
        }
    }

    /// The tide moved: the lane revision advances, the cards are told, and
    /// a ground machine caught on a cell that turned deep is sent to the
    /// nearest ground it can stand on.
    fn tide_changed(&mut self, text: &'static str) {
        self.gate.lane_revision = self.gate.lane_revision.saturating_add(1);
        self.lane_revision = self.gate.lane_revision;
        // The Salters' crust holds only until the tide next moves.
        if !self.crust.is_empty() {
            let melted = self.crust.len();
            self.crust.clear();
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::CrustMelted,
                player: None,
                entity: None,
                other: None,
                from: None,
                to: None,
                amount: melted as i32,
                text: "crust melted".to_string(),
                cause: None,
            });
        }
        self.events.push(Event::text(
            self.tick,
            EventKind::GateChanged,
            self.gate.owner,
            text,
        ));
        self.events.push(Event::text(
            self.tick,
            EventKind::GateLocked,
            self.gate.owner,
            "gate locked",
        ));
        let caught: Vec<(u32, u8, Pos)> = self
            .entities
            .iter()
            .filter(|e| {
                e.hp > 0
                    && !e.kind.is_building()
                    && e.aboard.is_none()
                    && e.kind != Kind::Dredger
                    && !matches!(spec(e.kind).movement, Movement::Hull | Movement::Air)
                    && self.deep_tidal(e.pos.cell_xy())
            })
            .map(|e| (e.id, e.owner, e.pos))
            .collect();
        for (id, owner, pos) in caught {
            let Some(shore) = self.nearest_ground_for(id, pos) else {
                continue;
            };
            if let Some(entity) = self.entity_mut(id) {
                entity.order = Order::Move { target: shore };
                entity.waypoints.clear();
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
                entity.after_pack = None;
                entity.deployed = false;
                entity.deploy_remaining = 0;
                entity.deploy_target = false;
            }
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::Swamped,
                Some(owner),
                id,
                "swamped: making for the shore",
            ));
        }
    }

    /// The nearest cell a machine may stand on at this tide, searched in
    /// rings out to twelve cells.
    fn nearest_ground_for(&self, id: u32, from: Pos) -> Option<Pos> {
        let entity = self.entity(id)?;
        let (cx, cy) = from.cell_xy();
        for radius in 1i32..=12 {
            let mut best: Option<(i64, Pos)> = None;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let cell = (cx + dx, cy + dy);
                    if !self.cell_walkable_for(entity.kind, cell) {
                        continue;
                    }
                    let pos = Pos::cell(cell.0, cell.1);
                    let d = pos.distance_sq(from);
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, pos));
                    }
                }
            }
            if let Some((_, pos)) = best {
                return Some(pos);
            }
        }
        None
    }

    /// Boarding and riding: a machine beside its transport with room climbs
    /// aboard and leaves the field; a machine aboard rides at the
    /// transport's position.
    fn update_boarding(&mut self) {
        let boarding: Vec<(u32, u32)> = self
            .entities
            .iter()
            .filter_map(|entity| match entity.order {
                Order::Board { transport } if entity.hp > 0 && entity.aboard.is_none() => {
                    Some((entity.id, transport))
                }
                _ => None,
            })
            .collect();
        let radius = i64::from(FP * bw_content::BOARD_RADIUS_CELLS).pow(2);
        for (id, transport) in boarding {
            let Some(carrier) = self.entity(transport).cloned() else {
                self.clear_order(id);
                continue;
            };
            let Some(rider) = self.entity(id).cloned() else {
                continue;
            };
            if carrier.hp <= 0 || carrier.owner != rider.owner || !carrier.kind.is_transport() {
                self.clear_order(id);
                continue;
            }
            if rider.pos.distance_sq(carrier.pos) > radius {
                continue;
            }
            if carrier.cargo.len() >= bw_content::TRANSPORT_CAPACITY {
                self.clear_order(id);
                self.events.push(Event::with_entity(
                    self.tick,
                    EventKind::PathBlocked,
                    Some(rider.owner),
                    id,
                    "transport full",
                ));
                continue;
            }
            if let Some(entity) = self.entity_mut(id) {
                entity.aboard = Some(transport);
                entity.pos = carrier.pos;
                entity.order = Order::Idle;
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
                entity.waypoints.clear();
                entity.deployed = false;
                entity.deploy_remaining = 0;
                entity.deploy_target = false;
            }
            if let Some(entity) = self.entity_mut(transport) {
                entity.cargo.push(id);
            }
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::Boarded,
                Some(rider.owner),
                id,
                "boarded",
            ));
        }
        // Riders keep their transport's position.
        let carriers: Vec<(u32, Pos)> = self
            .entities
            .iter()
            .filter(|e| e.hp > 0 && !e.cargo.is_empty())
            .map(|e| (e.id, e.pos))
            .collect();
        for (transport, pos) in carriers {
            for entity in &mut self.entities {
                if entity.aboard == Some(transport) {
                    entity.pos = pos;
                }
            }
        }
    }

    /// Set a transport's hold on free ground within reach; returns how many
    /// landed.  Riders with no ground to stand on stay aboard.
    fn unload(&mut self, transport: u32) -> usize {
        let Some(carrier) = self.entity(transport).cloned() else {
            return 0;
        };
        let (cx, cy) = carrier.pos.cell_xy();
        let lane_revision = self.gate.lane_revision;
        let mut landed = 0;
        let mut taken: Vec<(i32, i32)> = Vec::new();
        for id in carrier.cargo.clone() {
            let Some(rider) = self.entity(id).cloned() else {
                continue;
            };
            let mut spot = None;
            'search: for radius in 1i32..=bw_content::UNLOAD_RADIUS_CELLS {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx.abs() != radius && dy.abs() != radius {
                            continue;
                        }
                        let cell = (cx + dx, cy + dy);
                        if taken.contains(&cell)
                            || !self.cell_walkable_for(rider.kind, cell)
                            || self.occupied_by_other(id, cell)
                        {
                            continue;
                        }
                        spot = Some(cell);
                        break 'search;
                    }
                }
            }
            let Some(cell) = spot else {
                continue;
            };
            taken.push(cell);
            if let Some(entity) = self.entity_mut(id) {
                entity.aboard = None;
                entity.pos = Pos::cell(cell.0, cell.1);
                entity.order = Order::Idle;
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = None;
                entity.path_lane_revision = lane_revision;
            }
            if let Some(entity) = self.entity_mut(transport) {
                entity.cargo.retain(|c| *c != id);
            }
            self.events.push(Event::with_entity(
                self.tick,
                EventKind::Unloaded,
                Some(carrier.owner),
                id,
                "unloaded",
            ));
            landed += 1;
        }
        landed
    }

    /// A transport that dies takes its hold with it.
    fn sink_cargo(&mut self) {
        let lost: Vec<u32> = self
            .entities
            .iter()
            .filter(|e| e.hp <= 0 && !e.cargo.is_empty())
            .flat_map(|e| e.cargo.iter().copied())
            .collect();
        for id in lost {
            let Some(rider) = self.entity(id).cloned() else {
                continue;
            };
            if rider.hp <= 0 {
                continue;
            }
            if let Some(entity) = self.entity_mut(id) {
                entity.hp = 0;
            }
            self.players[rider.owner as usize].crew = self.players[rider.owner as usize]
                .crew
                .saturating_sub(spec(rider.kind).crew);
            let cause = rider
                .aboard
                .and_then(|transport| self.entity(transport))
                .map(|transport| transport.kind);
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::Death,
                player: Some(rider.owner),
                entity: Some(rider.id),
                other: rider.aboard,
                from: Some(rider.pos),
                to: None,
                amount: 0,
                text: rider.kind.name().to_string(),
                cause,
            });
        }
    }

    /// The arm after the dry one, in the map's order: what a plain switch
    /// opens. On the Split Basin, the other side.
    fn next_arm(&self) -> Arm {
        let arms = self.map.layout().arm_count().max(1);
        Arm(((self.gate.dry_arm.index() + 1) % arms) as u8)
    }

    /// The water on a tidal terrain at this tide; None for other ground.
    pub fn depth(&self, terrain: Terrain) -> Option<Depth> {
        let arm = terrain.tidal_arm()?;
        Some(match self.gate.tide {
            Tide::Neutral => Depth::Shallow,
            Tide::Flood => Depth::Deep,
            Tide::Open => {
                if arm == self.gate.dry_arm.0 {
                    Depth::Dry
                } else {
                    Depth::Deep
                }
            }
        })
    }

    /// The water under a cell: a crusted tidal cell is dry at any tide.
    pub fn depth_at(&self, x: i32, y: i32) -> Option<Depth> {
        let depth = self.depth(self.map.terrain(x, y))?;
        if self.is_crusted(x, y) {
            Some(Depth::Dry)
        } else {
            Some(depth)
        }
    }

    /// Whether a Salter's crust lies on this cell now.
    pub fn is_crusted(&self, x: i32, y: i32) -> bool {
        let (width, height) = (i32::from(self.map.width), i32::from(self.map.height));
        !self.crust.is_empty()
            && (0..width).contains(&x)
            && (0..height).contains(&y)
            && self.crust.binary_search(&((y * width + x) as u32)).is_ok()
    }

    fn has_headquarters(&self, seat: u8) -> bool {
        self.entities
            .iter()
            .any(|entity| entity.owner == seat && entity.kind == Kind::Headquarters)
    }

    /// Seats still in the match: not knocked out, with a headquarters.
    pub fn standing_seats(&self) -> Vec<u8> {
        self.seats()
            .filter(|&seat| !self.is_eliminated(seat) && self.has_headquarters(seat))
            .collect()
    }

    /// After a seat goes out, end the match if one seat or none is left.
    fn settle_last_standing(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        let standing = self.standing_seats();
        let outcome = match standing[..] {
            [winner] => Outcome::Victory(winner),
            [] => Outcome::Draw,
            _ => return,
        };
        let text = match outcome {
            Outcome::Victory(winner) => {
                format!("{} WINS", self.players[winner as usize].faction.name())
            }
            Outcome::Draw => "THE LAST HEADQUARTERS FELL TOGETHER".to_string(),
        };
        let kind = if matches!(outcome, Outcome::Draw) {
            EventKind::Draw
        } else {
            EventKind::Victory
        };
        self.outcome = Some(outcome);
        self.events.push(Event::text(self.tick, kind, None, text));
    }

    /// Knock `seat` out of a match of three or more: its machines become
    /// wrecks where they stand, its buildings fall, its orders stop, and
    /// the station is freed if it held it.
    fn eliminate_seat(&mut self, seat: u8, why: &str) {
        if self.is_eliminated(seat) || usize::from(seat) >= self.players.len() {
            return;
        }
        self.eliminated.push((seat, self.tick));
        let fallen: Vec<Entity> = self
            .entities
            .iter()
            .filter(|entity| entity.owner == seat && entity.hp > 0)
            .cloned()
            .collect();
        for entity in &fallen {
            if entity.aboard.is_none() {
                self.leave_scrap_at(entity, SCRAP_BASE_PERCENT);
            }
        }
        for entity in &mut self.entities {
            if entity.owner == seat {
                entity.hp = 0;
            }
        }
        self.entities.retain(|entity| entity.hp > 0);
        self.artillery.retain(|shot| shot.owner != seat);
        self.pending.retain(|scheduled| scheduled.player != seat);
        self.beacons.retain(|beacon| beacon.owner != seat);
        let side = &mut self.players[seat as usize];
        side.crew = 0;
        side.research = None;
        side.vent_remaining = 0;
        self.lane_hold[seat as usize] = 0;
        if self.gate.owner == Some(seat) {
            self.gate.owner = None;
        }
        if self.gate.capture_player == Some(seat) {
            self.gate.capture_player = None;
            self.gate.capture_progress = 0;
            self.gate.capture_missing_ticks = 0;
        }
        let name = self.players[seat as usize].faction.name();
        self.events.push(Event::text(
            self.tick,
            EventKind::Eliminated,
            Some(seat),
            format!("{name} IS OUT: {why}"),
        ));
    }

    fn update_victory(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        // A seat whose headquarters has fallen is out. While two or more
        // still stand the match goes on without it; otherwise the one left
        // wins, or the last ones fell together and it is a draw. With two
        // seats this is the old rule exactly, and nobody is ever recorded
        // as knocked out.
        let fallen: Vec<u8> = self
            .seats()
            .filter(|&seat| !self.is_eliminated(seat) && !self.has_headquarters(seat))
            .collect();
        let standing = self.standing_seats();
        if standing.len() >= 2 {
            for seat in fallen {
                self.eliminate_seat(seat, "HEADQUARTERS FELL");
            }
        } else if let [winner] = standing[..] {
            self.outcome = Some(Outcome::Victory(winner));
        } else {
            self.outcome = Some(Outcome::Draw);
        }
        let mut tide = false;
        if self.outcome.is_none() {
            for player in self.seats() {
                if self.lane_hold[player as usize] >= self.hold_ticks() {
                    self.outcome = Some(Outcome::Victory(player));
                    tide = true;
                    break;
                }
            }
        }
        if let Some(outcome) = &self.outcome {
            let text = match outcome {
                Outcome::Victory(winner) if tide => {
                    format!(
                        "{} HOLDS THE TIDE",
                        self.players[*winner as usize].faction.name()
                    )
                }
                Outcome::Victory(winner) => {
                    format!("{} WINS", self.players[*winner as usize].faction.name())
                }
                Outcome::Draw if self.players.len() == 2 => "BOTH HEADQUARTERS FELL".to_string(),
                Outcome::Draw => "THE LAST HEADQUARTERS FELL TOGETHER".to_string(),
            };
            self.events.push(Event::text(
                self.tick,
                if matches!(outcome, Outcome::Draw) {
                    EventKind::Draw
                } else {
                    EventKind::Victory
                },
                None,
                text,
            ));
        }
    }

    fn update_visibility_memory(&mut self) {
        let tick = self.tick;
        for player in self.seats() {
            let snapshots: Vec<KnowledgeEntity> = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.owner != player
                        && entity.hp > 0
                        && entity.build_remaining == 0
                        && self.entity_visible(player, entity.id)
                })
                .map(|entity| KnowledgeEntity {
                    id: entity.id,
                    owner: entity.owner,
                    kind: entity.kind,
                    pos: entity.pos,
                    hp: Some(entity.hp),
                    last_seen: tick,
                })
                .collect();
            for snapshot in snapshots {
                if let Some(existing) = self.observations[player as usize]
                    .iter_mut()
                    .find(|observation| observation.id == snapshot.id)
                {
                    *existing = snapshot.clone();
                } else {
                    self.observations[player as usize].push(snapshot.clone());
                }
                if let Some(entity) = self.entity_mut(snapshot.id) {
                    entity.last_seen = Some(tick);
                }
            }
            self.observations[player as usize].sort_by_key(|observation| observation.id);
            if self.observations[player as usize].len() > MAX_ENTITIES {
                self.observations[player as usize].truncate(MAX_ENTITIES);
            }
        }
    }

    /// The practice AI's turn: economy, production, the tier-two roles from
    /// the Drydock, the tide plan, and the army.  Every decision is read off
    /// the current state and every order goes through the normal command
    /// path, so a replay of the same seed reproduces it exactly.
    fn run_ai(&mut self) {
        if !self.ai_enabled
            || self.tick < self.ai_last_tick.saturating_add(self.ai_level.interval())
        {
            return;
        }
        self.ai_last_tick = self.tick;
        // The practice AI plays every seat but the person's (seat 0): one
        // opponent on the Split Basin, two on the Confluence.
        for player in self.seats().skip(1) {
            if !self.is_eliminated(player) {
                self.run_ai_seat(player);
            }
        }
    }

    /// One practice-AI turn for `player`, the person's seat included.
    /// For demonstrations only (the trailer capture lets the AI play every
    /// seat); a match never calls it. Call it no more often than every 15
    /// ticks, the AI's own pace.
    pub fn ai_turn_for(&mut self, player: u8) {
        if usize::from(player) < self.seat_count() && !self.is_eliminated(player) {
            self.run_ai_seat(player);
        }
    }

    fn run_ai_seat(&mut self, player: u8) {
        let own: Vec<Entity> = self
            .entities
            .iter()
            .filter(|entity| entity.owner == player && entity.hp > 0 && entity.aboard.is_none())
            .cloned()
            .collect();
        if own.is_empty() {
            return;
        }
        let knowledge = self.knowledge(player);
        let faction = self.players[player as usize].faction;

        self.ai_headquarters(player, &own, faction);
        self.ai_construction(player, &own, faction);
        self.ai_production(player, &own, faction);
        self.ai_gather(player, &own, &knowledge);
        self.ai_watch(player, &own);
        self.ai_mend(player, &own);
        let mut reserved = self.ai_tide(player, &own);
        if faction == Faction::Compact {
            self.ai_compact(player, &own);
        }
        self.ai_ferry(player, &own, &mut reserved);
        self.ai_army(player, &own, &knowledge.visible, faction, &reserved);
    }

    /// Whether the practice AI can pay for a machine and has the crew for it.
    fn ai_can_afford(&self, player: u8, kind: Kind) -> bool {
        let content = spec(kind);
        let side = &self.players[player as usize];
        side.salvage >= content.salvage
            && side.pressure >= content.pressure
            && side.crew.saturating_add(content.crew) <= side.cap
    }

    /// How many machines of a kind the practice AI has standing or on order.
    fn ai_fielded(&self, player: u8, kind: Kind) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.owner == player && entity.hp > 0)
            .map(|entity| {
                usize::from(entity.kind == kind)
                    .saturating_add(entity.queue.iter().filter(|item| item.kind == kind).count())
            })
            .sum()
    }

    /// Where the army goes with no headquarters in sight: the first other
    /// seat's starting headquarters (the west bank's on the Split Basin).
    fn ai_fallback_target(&self, player: u8) -> Pos {
        let layout = self.map.layout();
        let target = self
            .seats()
            .find(|&seat| seat != player && !self.is_eliminated(seat))
            .unwrap_or(0);
        layout.headquarters(target)
    }

    /// Where the army goes with three or more seats. A count first: an
    /// opponent holding the station with its gauge running is stopped at
    /// the mouth between the two banks, whoever it is winning against.
    /// Otherwise the weakest opponent it can walk to (the arm between them
    /// is not deep), by the hull of its machines in sight; and with none in
    /// reach, it waits at its own mouth toward the weakest for the tide.
    fn ai_target(&self, player: u8, visible_enemy: &[KnowledgeEntity]) -> Pos {
        let layout = self.map.layout();
        let opponents: Vec<u8> = self
            .standing_seats()
            .into_iter()
            .filter(|&seat| seat != player)
            .collect();
        let toward = |seat: u8| {
            layout
                .arm_between(player, seat)
                .map(|arm| self.ai_near_mouth(player, arm))
        };
        if let Some(counting) = opponents
            .iter()
            .copied()
            .find(|&seat| self.gate.owner == Some(seat) && self.lane_hold[usize::from(seat)] > 0)
            && let Some(mouth) = toward(counting)
        {
            return mouth;
        }
        // Someone else's machines on a station nobody owns: clear it, or
        // two channelers stand side by side and nobody ever takes it.
        let station = self.map.gate_pos;
        let reach_station = layout
            .arms_of(player)
            .any(|arm| self.depth(Terrain::lane(arm as u8)) != Some(Depth::Deep));
        if self.gate.owner.is_none() && reach_station && self.station_contested_for(player) {
            return station;
        }
        // Holding the station, a lane of its own is denied at the far end
        // by the seat across the arm: clear that mouth while the lane can be
        // walked, or no count ever starts.
        if self.gate.owner == Some(player)
            && let Some(far) = self.ai_denied_far_mouth(player)
        {
            return far;
        }
        let reachable = |seat: u8| {
            layout
                .arm_between(player, seat)
                .is_none_or(|arm| self.depth(Terrain::lane(arm as u8)) != Some(Depth::Deep))
        };
        let strength = |seat: u8| -> i64 {
            visible_enemy
                .iter()
                .filter(|entity| entity.owner == seat && is_combat_unit(entity.kind))
                .map(|entity| i64::from(entity.hp.unwrap_or(0)))
                .sum()
        };
        let pick = |candidates: Vec<u8>| {
            candidates
                .into_iter()
                .min_by_key(|&seat| (strength(seat), seat))
        };
        let in_reach: Vec<u8> = opponents
            .iter()
            .copied()
            .filter(|&seat| reachable(seat))
            .collect();
        let headquarters = |seat: u8| {
            visible_enemy
                .iter()
                .find(|entity| entity.owner == seat && entity.kind == Kind::Headquarters)
                .map_or_else(|| layout.headquarters(seat), |entity| entity.pos)
        };
        if let Some(seat) = pick(in_reach) {
            return headquarters(seat);
        }
        match pick(opponents) {
            Some(seat) => toward(seat).unwrap_or_else(|| headquarters(seat)),
            None => self.ai_fallback_target(player),
        }
    }

    /// With three seats: one of the practice AI's own lanes whose far
    /// mouth another seat's holder denies, so no hold can start.
    fn ai_denied_arm(&self, player: u8) -> Option<usize> {
        if self.seat_count() <= 2 {
            return None;
        }
        // A garrison steps off its mouth to fight and comes back, so an
        // enemy holder anywhere near the far mouth counts. An enemy Defense
        // Nest in the ring only slows a three-seat count (rules 22), so it
        // is no denial.
        let near = i64::from(FP * AI_DENIAL_CELLS).pow(2);
        self.ai_arms(player).into_iter().find(|&arm| {
            let far = self.ai_far_mouth(player, arm);
            self.mouth_blocked_for(player, far)
                || self.entities.iter().any(|entity| {
                    entity.owner != player
                        && entity.hp > 0
                        && entity.build_remaining == 0
                        && entity.aboard.is_none()
                        && holds_mouth_unit(entity.kind)
                        && entity.pos.distance_sq(far) <= near
                })
        })
    }

    /// The far mouth of a denied lane (`ai_denied_arm`), while the lane can
    /// be walked.
    fn ai_denied_far_mouth(&self, player: u8) -> Option<Pos> {
        let arm = self.ai_denied_arm(player)?;
        (self.depth(Terrain::lane(arm as u8)) != Some(Depth::Deep))
            .then(|| self.ai_far_mouth(player, arm))
    }

    /// The arms whose lanes the practice AI must hold, in map order: those
    /// that touch its land, less a lane to a seat that is out (rules 20,
    /// `hold_arms`).
    fn ai_arms(&self, player: u8) -> Vec<usize> {
        let arms = self.hold_arms(player);
        if arms.is_empty() {
            self.map.layout().arms_of(player).collect()
        } else {
            arms
        }
    }

    /// The crossing mouth of each of its lanes on the practice AI's own
    /// bank, in the order of `ai_arms`: the end it can garrison without
    /// crossing the water.
    fn ai_mouths(&self, player: u8) -> Vec<Pos> {
        self.ai_arms(player)
            .into_iter()
            .map(|arm| self.ai_near_mouth(player, arm))
            .collect()
    }

    /// The mouth of an arm's lane on the practice AI's own bank.
    fn ai_near_mouth(&self, player: u8, arm: usize) -> Pos {
        let layout = self.map.layout();
        let side = usize::from(layout.arms[arm].banks[1] == player);
        layout.arm_mouths(arm)[side]
    }

    /// The far end of a lane's crossing: the mouth on the enemy's bank.
    fn ai_far_mouth(&self, player: u8, arm: usize) -> Pos {
        let layout = self.map.layout();
        let side = usize::from(layout.arms[arm].banks[1] == player);
        layout.arm_mouths(arm)[1 - side]
    }

    /// The headquarters: keep a worker line coming and take a doctrine once
    /// the base can pay for one.
    fn ai_headquarters(&mut self, player: u8, own: &[Entity], faction: Faction) {
        let Some(hq) = own
            .iter()
            .find(|entity| entity.kind == Kind::Headquarters && entity.build_remaining == 0)
            .cloned()
        else {
            return;
        };
        let worker_count = own.iter().filter(|entity| entity.kind.is_worker()).count();
        if self.players[player as usize].doctrine.is_none()
            && self.players[player as usize].research.is_none()
            && self.players[player as usize].salvage >= DOCTRINE_SALVAGE
            && self.players[player as usize].pressure >= DOCTRINE_PRESSURE
            && self.tick >= 900
        {
            let doctrine = if worker_count < self.ai_level.worker_target() - 1 {
                Doctrine::Hauling
            } else {
                Doctrine::FireControl
            };
            let _ = self.ai_issue(
                player,
                Command::Research {
                    building: hq.id,
                    doctrine,
                },
            );
        }
        // RECLAIM: the practice AI banks more pressure than it spends, and
        // salvage is what it is always short of.  Keep the flood's price
        // back, then turn the rest into salvage.
        if self.players[player as usize].pressure >= RECLAIM_PRESSURE.saturating_add(FLOOD_PRESSURE)
            && self.players[player as usize].salvage < AI_RECLAIM_FLOOR
        {
            let _ = self.ai_issue(player, Command::Reclaim { building: hq.id });
        }
        self.ai_overhaul(player, own, &hq);
        if worker_count < self.ai_level.worker_target()
            && hq.queue.is_empty()
            && self.ai_can_afford(player, faction.worker())
        {
            let _ = self.ai_issue(
                player,
                Command::Train {
                    building: hq.id,
                    kind: faction.worker(),
                },
            );
        }
    }

    /// OVERHAUL (rules 21): the practice AI buys the next level once it has
    /// banked more than twice its price and its Works is not starved for
    /// salvage, that is, the Works has a queue or the crew is full.
    fn ai_overhaul(&mut self, player: u8, own: &[Entity], hq: &Entity) {
        if hq.upgrade.is_some() || !hq.upgrade_queue.is_empty() {
            return;
        }
        let side = &self.players[player as usize];
        let Some(next) = Upgrade::OVERHAUL
            .into_iter()
            .find(|level| !side.upgrades.contains(level))
        else {
            return;
        };
        let crew_full = side.crew >= side.cap;
        let works_busy = own.iter().any(|entity| {
            entity.kind == Kind::Works && entity.build_remaining == 0 && !entity.queue.is_empty()
        });
        if side.salvage > next.cost().0.saturating_mul(2) && (works_busy || crew_full) {
            let _ = self.ai_issue(
                player,
                Command::Upgrade {
                    building: hq.id,
                    upgrade: next,
                },
            );
        }
    }

    /// Construction: the economy first, then the Drydock, which is what
    /// opens the tier-two roles and with them the tide plan.
    fn ai_construction(&mut self, player: u8, own: &[Entity], faction: Faction) {
        let standing = |kind: Kind| own.iter().any(|entity| entity.kind == kind);
        let sites = &self.map.layout().seats[usize::from(player)].ai;
        let site = |(x, y): (i32, i32)| Pos::cell(x, y);
        let next = if !standing(Kind::Works) {
            Some((Kind::Works, site(sites.works)))
        } else if !standing(Kind::Dropoff) {
            Some((Kind::Dropoff, site(sites.dropoff)))
        } else if !standing(Kind::Condenser) {
            Some((Kind::Condenser, site(sites.condenser)))
        } else if !standing(Kind::Drydock) {
            self.ai_drydock_site(player, faction)
                .map(|pos| (Kind::Drydock, pos))
        } else {
            None
        };
        let Some((kind, pos)) = next else {
            return;
        };
        if self.players[player as usize].salvage < spec(kind).salvage || !self.can_place(kind, pos)
        {
            return;
        }
        let Some(worker) = own.iter().find(|entity| {
            entity.kind.is_worker() && matches!(entity.order, Order::Gather { .. } | Order::Idle)
        }) else {
            return;
        };
        let worker = worker.id;
        let _ = self.ai_issue(
            player,
            Command::Build {
                worker,
                kind,
                pos,
                queued: false,
            },
        );
    }

    /// The salvage the practice AI holds back from the Works for whatever it
    /// is saving for next: the building it still needs, or the tier-two role
    /// the Drydock has not turned out yet.  Without it the Works eats every
    /// scrap the workers bring in and the base never grows past its yard.
    fn ai_reserve(&self, player: u8, own: &[Entity], faction: Faction) -> u32 {
        let standing = |kind: Kind| own.iter().any(|entity| entity.kind == kind);
        if !standing(Kind::Works) || !standing(Kind::Dropoff) {
            return 0;
        }
        if !standing(Kind::Condenser) {
            return spec(Kind::Condenser).salvage;
        }
        if !standing(Kind::Drydock) {
            return if self.ai_drydock_site(player, faction).is_some() {
                spec(Kind::Drydock).salvage
            } else {
                0
            };
        }
        self.ai_wanted_role(player, faction)
            .map_or(0, |kind| self.ai_role_price(kind))
    }

    /// The first Drydock site beside the practice AI's base that takes the
    /// building and, for a faction whose transport is a hull, has water
    /// within launching reach.
    fn ai_drydock_site(&self, player: u8, faction: Faction) -> Option<Pos> {
        let hull = faction
            .drydock_roles()
            .into_iter()
            .find(|kind| matches!(spec(*kind).movement, Movement::Hull));
        let sites = self.map.layout().seats[usize::from(player)].ai.drydocks;
        sites.into_iter().map(|(x, y)| Pos::cell(x, y)).find(|pos| {
            self.can_place(Kind::Drydock, *pos)
                && hull.is_none_or(|kind| self.find_spawn_pos_for(kind, *pos).is_some())
        })
    }

    /// Production: the Works keeps an army coming, the Drydock fills out the
    /// tier-two roles, and TRACKS pays for itself once the AI is drowning
    /// its own lanes.
    fn ai_production(&mut self, player: u8, own: &[Entity], faction: Faction) {
        let reserve = self.ai_reserve(player, own, faction);
        if let Some(works) = own
            .iter()
            .find(|entity| entity.kind == Kind::Works && entity.build_remaining == 0)
            .cloned()
            && works.queue.len() < 5
        {
            let army = faction.army();
            let preferred = (self.tick / 90) as usize % army.len();
            let choice = (0..army.len())
                .map(|offset| army[(preferred + offset) % army.len()])
                .find(|kind| {
                    self.ai_can_afford(player, *kind)
                        && self.players[player as usize].salvage
                            >= spec(*kind).salvage.saturating_add(reserve)
                });
            if let Some(kind) = choice {
                let _ = self.ai_issue(
                    player,
                    Command::Train {
                        building: works.id,
                        kind,
                    },
                );
            }
        }
        let Some(drydock) = own
            .iter()
            .find(|entity| entity.kind == Kind::Drydock && entity.build_remaining == 0)
            .cloned()
        else {
            return;
        };
        if drydock.queue.len() < 3
            && let Some(kind) = self.ai_drydock_role(player, faction)
        {
            let _ = self.ai_issue(
                player,
                Command::Train {
                    building: drydock.id,
                    kind,
                },
            );
        }
        // TRACKS once the roles are out and the sluice is this side's: the
        // practice AI drowns its own lanes, and halving the mire is what
        // lets its garrison walk through one.
        let (salvage, pressure, _) = Upgrade::Tracks.cost();
        if drydock.upgrade.is_none()
            && !self.players[player as usize]
                .upgrades
                .contains(&Upgrade::Tracks)
            && self.ai_wanted_role(player, faction).is_none()
            && self.gate.owner == Some(player)
            && self.players[player as usize].salvage >= salvage
            && self.players[player as usize].pressure >= pressure.saturating_add(FLOOD_PRESSURE)
        {
            let _ = self.ai_issue(
                player,
                Command::Upgrade {
                    building: drydock.id,
                    upgrade: Upgrade::Tracks,
                },
            );
        }
    }

    /// The tier-two role the practice AI is short of, whether or not it can
    /// pay for one yet: a scout to watch the mouths, a mender for the army,
    /// a pair of the faction's water role, then a transport.  The order is
    /// also the order it saves in, so the answer must not depend on what is
    /// in the bank.
    fn ai_wanted_role(&self, player: u8, faction: Faction) -> Option<Kind> {
        faction
            .drydock_roles()
            .into_iter()
            .zip(AI_DRYDOCK_ROLE_WANTED)
            .find(|(kind, wanted)| self.ai_fielded(player, *kind) < *wanted)
            .map(|(kind, _)| kind)
    }

    /// The salvage a role costs the practice AI, transports included: the
    /// hold is a luxury, so it waits until the army is not short as well.
    fn ai_role_price(&self, kind: Kind) -> u32 {
        if kind.is_transport() {
            spec(kind).salvage.saturating_add(AI_TRANSPORT_RESERVE)
        } else {
            spec(kind).salvage
        }
    }

    /// The role the Drydock can start on this tick.
    fn ai_drydock_role(&self, player: u8, faction: Faction) -> Option<Kind> {
        let kind = self.ai_wanted_role(player, faction)?;
        (self.ai_can_afford(player, kind)
            && self.players[player as usize].salvage >= self.ai_role_price(kind))
        .then_some(kind)
    }

    /// Salvage: idle gatherers take the nearest wreck they can work, and the
    /// Dredger prefers the drowned lane wrecks, which is the one bed no
    /// worker can reach.
    fn ai_gather(&mut self, player: u8, own: &[Entity], knowledge: &PlayerKnowledge) {
        let gatherers: Vec<Entity> = own
            .iter()
            .filter(|entity| entity.kind.gathers() && matches!(entity.order, Order::Idle))
            .cloned()
            .collect();
        for gatherer in gatherers {
            let prefers_wet = gatherer.kind == Kind::Dredger;
            let resource = knowledge
                .map
                .resources
                .iter()
                .filter(|resource| {
                    resource.kind == ResourceKind::Salvage
                        && resource.remaining > 0
                        && self.gatherable(gatherer.kind, resource)
                })
                .min_by_key(|resource| {
                    let (x, y) = resource.pos.cell_xy();
                    let drowned = prefers_wet
                        && matches!(self.depth_at(x, y), Some(Depth::Shallow | Depth::Deep));
                    (
                        !drowned,
                        gatherer.pos.distance_sq(resource.pos),
                        resource.id,
                    )
                })
                .map(|resource| resource.id);
            if let Some(resource) = resource {
                let _ = self.ai_issue(
                    player,
                    Command::Gather {
                        units: vec![gatherer.id],
                        resource,
                    },
                );
            }
        }
    }

    /// The tier-two scouts: each takes station at a crossing mouth and
    /// holds there, where its sight doubles and the lane is watched.
    fn ai_watch(&mut self, player: u8, own: &[Entity]) {
        let mouths = self.ai_mouths(player);
        let reach = i64::from(FP * AI_GARRISON_CELLS).pow(2);
        let scouts: Vec<Entity> = own
            .iter()
            .filter(|entity| entity.kind.is_scout())
            .cloned()
            .collect();
        for (index, scout) in scouts.iter().enumerate() {
            let post = mouths[index % mouths.len()];
            if scout.pos.distance_sq(post) <= reach {
                if !matches!(scout.order, Order::Hold) {
                    let _ = self.ai_issue(
                        player,
                        Command::Hold {
                            units: vec![scout.id],
                        },
                    );
                }
            } else if matches!(scout.order, Order::Idle | Order::Hold) {
                let _ = self.ai_issue(
                    player,
                    Command::Move {
                        units: vec![scout.id],
                        target: post,
                        queued: false,
                    },
                );
            }
        }
    }

    /// The menders: their aura is a three-cell circle, so they are only
    /// worth anything standing next to what is being shot.
    fn ai_mend(&mut self, player: u8, own: &[Entity]) {
        // With nothing hurt, trail the machine that has pushed furthest
        // toward the enemy bank, which is where the shooting starts.
        let fallback = own
            .iter()
            .filter(|entity| is_combat_unit(entity.kind))
            .min_by_key(|entity| {
                let (fx, fy) = self.map.layout().seats[usize::from(player)].ai.forward;
                let along = i64::from(entity.pos.x) * i64::from(fx)
                    + i64::from(entity.pos.y) * i64::from(fy);
                (-along, entity.id)
            })
            .map_or_else(
                || {
                    self.ai_mouths(player)
                        .first()
                        .copied()
                        .unwrap_or(self.map.gate_pos)
                },
                |entity| entity.pos,
            );
        let menders: Vec<Entity> = own
            .iter()
            // At rest, not only idle: a finished move keeps its Move order,
            // and a mender that only answered Idle walked once and then
            // stood where it stopped for the rest of the match, in a lane
            // it then kept its own side from flooding.
            .filter(|entity| entity.kind.repairs() && at_rest(entity))
            .cloned()
            .collect();
        for mender in menders {
            let target = own
                .iter()
                .filter(|entity| {
                    entity.id != mender.id
                        && !entity.kind.is_building()
                        && !entity.kind.is_worker()
                        && entity.hp < entity.max_hp
                })
                .min_by_key(|entity| (mender.pos.distance_sq(entity.pos), entity.id))
                .map_or(fallback, |entity| entity.pos);
            if mender.pos.distance_sq(target) <= i64::from(FP * 2).pow(2) {
                continue;
            }
            let _ = self.ai_issue(
                player,
                Command::Move {
                    units: vec![mender.id],
                    target,
                    queued: false,
                },
            );
        }
    }

    /// The tide plan: take the sluice, stand a garrison at both mouths on
    /// this bank, then drown the lanes so nothing of the enemy's can walk
    /// over and contest one while the hold gauge fills.  Returns the
    /// machines it has spoken for, which the army leaves alone.
    fn ai_tide(&mut self, player: u8, own: &[Entity]) -> Vec<u32> {
        let mut reserved: Vec<u32> = Vec::new();
        let holders: Vec<Entity> = own
            .iter()
            .filter(|entity| {
                holds_mouth_unit(entity.kind)
                    && entity.build_remaining == 0
                    && entity.surge_remaining == 0
            })
            .cloned()
            .collect();
        // A garrison that walks away holds nothing, so machines already on
        // station or at a mouth stay there whatever the force looks like.
        // Only sending fresh ones waits for a force worth splitting.
        let committed = holders.len() >= AI_TIDE_MINIMUM;
        let faction_scout = match self.players[player as usize].faction {
            Faction::Union => Kind::Sounder,
            Faction::Assembly => Kind::Skipper,
            Faction::Compact => Kind::Glinter,
        };
        let mouths = self.ai_mouths(player);
        // One machine holds a mouth; a second only goes there once the AI
        // has enough left over to keep pushing as well.
        let garrison = if holders.len() >= AI_HEAVY_GARRISON {
            AI_MOUTH_GARRISON
        } else {
            1
        };
        let mouth_reach = i64::from(FP * AI_GARRISON_CELLS).pow(2);
        let station = self.map.gate_pos;
        let station_reach = i64::from(FP * 2).pow(2);

        // The station first: rules 12 make it part of the hold, so without
        // it both banks deny each other and the tide can never be won.
        let mut keepers: Vec<u32> = holders
            .iter()
            .filter(|entity| entity.pos.distance_sq(station) <= station_reach)
            .map(|entity| entity.id)
            .collect();
        keepers.sort_unstable();
        reserved.extend(keepers.iter().copied());
        // A machine already walking to the station is spoken for too, or a
        // bad trade at the enemy base drops the count and the army takes it
        // back before it ever arrives.
        let walking_in: Vec<u32> = holders
            .iter()
            .filter(
                |entity| matches!(entity.order, Order::AttackMove { target } if target == station),
            )
            .map(|entity| entity.id)
            .collect();
        reserved.extend(walking_in.iter().copied());
        if keepers.is_empty() && walking_in.is_empty() && holders.len() >= AI_STATION_MINIMUM {
            let runner = holders
                .iter()
                .filter(|entity| entity.kind != Kind::Caisson)
                .min_by_key(|entity| (entity.pos.distance_sq(station), entity.id))
                .map(|entity| (entity.id, entity.order.clone()));
            if let Some((id, order)) = runner {
                reserved.push(id);
                if !matches!(order, Order::AttackMove { target } if target == station) {
                    let _ = self.ai_issue(
                        player,
                        Command::AttackMove {
                            units: vec![id],
                            target: station,
                            queued: false,
                        },
                    );
                }
            }
        } else if self.gate.owner != Some(player)
            && self.seat_count() > 2
            && self.station_contested_for(player)
            && !keepers.is_empty()
        {
            // With three seats a second opponent's channeler on the station
            // freezes the capture for both, and channelers never fight: the
            // keepers turn on the nearest one instead. Two seats keep the
            // plan their replays were recorded with.
            let radius = i64::from(FP * 2).pow(2);
            let rival = self
                .entities
                .iter()
                .filter(|entity| {
                    entity.owner != player
                        && entity.hp > 0
                        && entity.build_remaining == 0
                        && is_combat_unit(entity.kind)
                        && entity.pos.distance_sq(station) <= radius
                })
                .min_by_key(|entity| (entity.hp, entity.id))
                .map(|entity| entity.id);
            if let Some(target) = rival {
                let idle: Vec<u32> = keepers
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.entity(*id).is_some_and(
                            |entity| !matches!(entity.order, Order::Attack { target: t } if t == target),
                        )
                    })
                    .collect();
                if !idle.is_empty() {
                    let _ = self.ai_issue(
                        player,
                        Command::Attack {
                            units: idle,
                            target,
                        },
                    );
                }
            }
        } else if self.gate.owner != Some(player) {
            let channeling: Vec<u32> = keepers
                .iter()
                .copied()
                .filter(|id| {
                    self.entity(*id)
                        .is_some_and(|entity| !matches!(entity.order, Order::Capture))
                })
                .collect();
            if !channeling.is_empty() {
                let _ = self.ai_issue(player, Command::Capture { units: channeling });
            }
        }

        // Then a garrison at each mouth on this bank.  A Caisson is the
        // mouth's own machine, so it goes first and digs in; the faction
        // scout goes last, because it is worth more on the map.
        for mouth in mouths.iter().copied() {
            let present: Vec<(u32, Kind, bool, u32)> = holders
                .iter()
                .filter(|entity| {
                    entity.pos.distance_sq(mouth) <= mouth_reach
                        || matches!(entity.order, Order::AttackMove { target } if target == mouth)
                })
                .map(|entity| {
                    (
                        entity.id,
                        entity.kind,
                        entity.deployed,
                        entity.deploy_remaining,
                    )
                })
                .collect();
            for (id, kind, deployed, deploying) in &present {
                if !reserved.contains(id) {
                    reserved.push(*id);
                }
                let arrived = self
                    .entity(*id)
                    .is_some_and(|entity| entity.pos.distance_sq(mouth) <= mouth_reach);
                if arrived && *kind == Kind::Caisson && !*deployed && *deploying == 0 {
                    let _ = self.ai_issue(player, Command::Deploy { units: vec![*id] });
                }
            }
            if !committed {
                continue;
            }
            let mut manned = present.len();
            // The Caisson is the mouth's own machine: one digs in at each
            // crossing before a gun machine is spared for it.
            if !present.iter().any(|(_, kind, _, _)| *kind == Kind::Caisson)
                && let Some((id, order)) = holders
                    .iter()
                    .filter(|entity| entity.kind == Kind::Caisson && !reserved.contains(&entity.id))
                    .min_by_key(|entity| (entity.pos.distance_sq(mouth), entity.id))
                    .map(|entity| (entity.id, entity.order.clone()))
            {
                reserved.push(id);
                manned = manned.saturating_add(1);
                if !matches!(order, Order::AttackMove { target } if target == mouth) {
                    let _ = self.ai_issue(
                        player,
                        Command::AttackMove {
                            units: vec![id],
                            target: mouth,
                            queued: false,
                        },
                    );
                }
            }
            for _ in manned..garrison {
                let pick = holders
                    .iter()
                    .filter(|entity| !reserved.contains(&entity.id))
                    .min_by_key(|entity| {
                        (
                            entity.kind != Kind::Caisson,
                            entity.kind == faction_scout,
                            entity.pos.distance_sq(mouth),
                            entity.id,
                        )
                    })
                    .map(|entity| (entity.id, entity.order.clone()));
                let Some((id, order)) = pick else {
                    break;
                };
                reserved.push(id);
                if !matches!(order, Order::AttackMove { target } if target == mouth) {
                    let _ = self.ai_issue(
                        player,
                        Command::AttackMove {
                            units: vec![id],
                            target: mouth,
                            queued: false,
                        },
                    );
                }
            }
        }

        // Finally the water itself.  Both mouths manned: drown the lanes,
        // which costs the enemy its only walk to either end of a crossing
        // while this side stands on dry bank.  Not manned yet: keep the
        // lane the force is waiting at open so it can cross at all.
        let manned = mouths.iter().all(|mouth| {
            holders
                .iter()
                .any(|entity| entity.pos.distance_sq(*mouth) <= mouth_reach)
        });
        if self.gate.owner != Some(player)
            || self.gate.warning_until.is_some()
            || self.tick < self.gate.locked_until
        {
            return reserved;
        }
        if manned {
            let wading = own.iter().any(|entity| {
                !entity.kind.is_building()
                    && entity.kind != Kind::Dredger
                    && !matches!(spec(entity.kind).movement, Movement::Air | Movement::Hull)
                    && {
                        let (x, y) = entity.pos.cell_xy();
                        self.map.terrain(x, y).is_tidal()
                    }
            });
            // With three seats a flood also drowns the walk to a far mouth
            // another seat holds, which the army must clear first: dry that
            // lane instead, so the army can walk over.
            let denied = self.ai_denied_arm(player);
            // Since rules 20 a flood freezes every count, this side's too,
            // so it no longer drowns empty lanes to guard a running count.
            // It drowns an attack caught wading toward a mouth instead:
            // swamped and slow, and the count keeps what it banked.
            let caught = self.ai_enemies_in_lanes(player);
            if let Some(arm) = denied {
                let arm = Arm(arm as u8);
                if self.gate.tide == Tide::Open
                    && self.gate.dry_arm != arm
                    && self.players[player as usize].pressure >= SWITCH_PRESSURE
                {
                    let _ = self.ai_issue(player, Command::SetTide { arm });
                }
                return reserved;
            }
            if !wading
                && caught >= AI_FLOOD_CATCH
                && self.gate.tide != Tide::Flood
                && self.players[player as usize].pressure
                    >= FLOOD_PRESSURE.saturating_add(SWITCH_PRESSURE)
            {
                let _ = self.ai_issue(player, Command::Flood);
            }
            return reserved;
        }
        if self.gate.tide == Tide::Open
            && self.players[player as usize].pressure
                >= SWITCH_PRESSURE.saturating_add(FLOOD_PRESSURE)
        {
            let layout = self.map.layout();
            let waiting = self
                .ai_arms(player)
                .into_iter()
                .zip(mouths.iter().copied())
                .find(|&(arm, mouth)| {
                    let (x, y) = layout.arms[arm].centre;
                    self.depth_at(x, y) == Some(Depth::Deep)
                        && holders
                            .iter()
                            .filter(|entity| {
                                entity.pos.distance_sq(mouth)
                                    <= i64::from(FP * AI_STAGING_CELLS).pow(2)
                            })
                            .count()
                            >= 2
                });
            if let Some((arm, _)) = waiting {
                let arm = Arm(arm as u8);
                if self.gate.dry_arm != arm {
                    let _ = self.ai_issue(player, Command::SetTide { arm });
                }
            }
        }
        reserved
    }

    /// Enemy gun machines the practice AI can see on tidal ground: what a
    /// flood would swamp.
    fn ai_enemies_in_lanes(&self, player: u8) -> usize {
        self.entities
            .iter()
            .filter(|entity| {
                entity.owner != player
                    && entity.hp > 0
                    && entity.aboard.is_none()
                    && is_combat_unit(entity.kind)
                    && !matches!(spec(entity.kind).movement, Movement::Air | Movement::Hull)
                    && self.entity_visible(player, entity.id)
                    && {
                        let (x, y) = entity.pos.cell_xy();
                        self.map.terrain(x, y).is_tidal()
                    }
            })
            .count()
    }

    /// The transport: while a lane is water nothing walks across it, so the
    /// hold carries a landing party to the mouth on the far bank, where it
    /// denies the enemy the other end of the crossing.
    fn ai_ferry(&mut self, player: u8, own: &[Entity], reserved: &mut Vec<u32>) {
        let Some(transport) = own
            .iter()
            .find(|entity| entity.kind.is_transport() && entity.build_remaining == 0)
            .cloned()
        else {
            return;
        };
        reserved.push(transport.id);
        // The crossing to work: the first whose far end this side is not
        // already holding, so the hold goes where the denial is missing.
        let arms = self.ai_arms(player);
        let Some(&first) = arms.first() else {
            return;
        };
        let lane = arms
            .into_iter()
            .find(|lane| !self.holds_crossing(player, self.ai_far_mouth(player, *lane)))
            .unwrap_or(first);
        let hull = matches!(spec(transport.kind).movement, Movement::Hull);
        let near = self.ai_near_mouth(player, lane);
        let far = self.ai_far_mouth(player, lane);
        let berth = |world: &World, beside: Pos| {
            if hull {
                world.find_spawn_pos_for(transport.kind, beside)
            } else {
                Some(beside)
            }
        };
        let board_reach = i64::from(FP * bw_content::BOARD_RADIUS_CELLS).pow(2);
        if transport.cargo.is_empty() {
            let Some(pickup) = berth(self, near) else {
                return;
            };
            if transport.pos.distance_sq(pickup) > board_reach {
                if !matches!(transport.order, Order::Move { target } if target == pickup) {
                    let _ = self.ai_issue(
                        player,
                        Command::Move {
                            units: vec![transport.id],
                            target: pickup,
                            queued: false,
                        },
                    );
                }
                return;
            }
            let mut riders: Vec<u32> = own
                .iter()
                .filter(|entity| {
                    is_combat_unit(entity.kind)
                        && !reserved.contains(&entity.id)
                        && !entity.deployed
                        && entity.deploy_remaining == 0
                        && entity.surge_remaining == 0
                        && !matches!(entity.order, Order::Board { .. })
                        && entity.pos.distance_sq(pickup)
                            <= i64::from(FP * AI_FERRY_CALL_CELLS).pow(2)
                })
                .map(|entity| entity.id)
                .take(bw_content::TRANSPORT_CAPACITY)
                .collect();
            riders.sort_unstable();
            if riders.is_empty() {
                return;
            }
            reserved.extend(riders.iter().copied());
            let _ = self.ai_issue(
                player,
                Command::Board {
                    units: riders,
                    transport: transport.id,
                },
            );
            return;
        }
        let walking = own.iter().any(
            |entity| matches!(entity.order, Order::Board { transport: hold } if hold == transport.id),
        );
        if walking && transport.cargo.len() < bw_content::TRANSPORT_CAPACITY {
            return;
        }
        let Some(drop) = berth(self, far) else {
            return;
        };
        // A hold that cannot reach the far bank sets its party down where
        // it stands rather than carrying it around for the rest of the
        // match.
        if transport.pos.distance_sq(drop) > board_reach
            && transport.blocked_ticks < AI_FERRY_GIVE_UP
        {
            if !matches!(transport.order, Order::Move { target } if target == drop) {
                let _ = self.ai_issue(
                    player,
                    Command::Move {
                        units: vec![transport.id],
                        target: drop,
                        queued: false,
                    },
                );
            }
            return;
        }
        let _ = self.ai_issue(
            player,
            Command::Unload {
                transport: transport.id,
            },
        );
    }

    /// The army: the faction scout reads the map, specialists set up on a
    /// target, and everything the tide plan has not spoken for pushes the
    /// enemy headquarters.
    fn ai_army(
        &mut self,
        player: u8,
        own: &[Entity],
        visible_enemy: &[KnowledgeEntity],
        faction: Faction,
        reserved: &[u32],
    ) {
        let scout_kind = match faction {
            Faction::Union => Kind::Sounder,
            Faction::Assembly => Kind::Skipper,
            Faction::Compact => Kind::Glinter,
        };
        let scouts: Vec<Entity> = own
            .iter()
            .filter(|entity| entity.kind == scout_kind && !reserved.contains(&entity.id))
            .cloned()
            .collect();
        let explore_points: Vec<Pos> = self.map.layout().seats[usize::from(player)]
            .ai
            .explore
            .iter()
            .map(|&(x, y)| Pos::cell(x, y))
            .collect();
        for scout in scouts {
            if scout.pos.distance_sq(self.map.gate_pos) <= i64::from(FP * 2).pow(2) {
                if self.gate.owner != Some(player) {
                    let _ = self.ai_issue(
                        player,
                        Command::Capture {
                            units: vec![scout.id],
                        },
                    );
                } else if scout.path.is_empty() && scout.path_target.is_none() {
                    let target = explore_points[(self.tick / 450) as usize % explore_points.len()];
                    let _ = self.ai_issue(
                        player,
                        Command::AttackMove {
                            units: vec![scout.id],
                            target,
                            queued: false,
                        },
                    );
                }
            } else if scout.path.is_empty() && scout.path_target.is_none() {
                let target = visible_enemy
                    .iter()
                    .find(|entity| entity.kind == Kind::Headquarters)
                    .map(|entity| entity.pos)
                    .unwrap_or(explore_points[(self.tick / 450) as usize % explore_points.len()]);
                let _ = self.ai_issue(
                    player,
                    Command::AttackMove {
                        units: vec![scout.id],
                        target,
                        queued: false,
                    },
                );
            }
        }
        let front_radius = i64::from(FP * 18).pow(2);
        for specialist in own.iter().filter(|entity| {
            is_specialist(entity.kind)
                && !matches!(entity.kind, Kind::Caisson | Kind::Pan)
                && !reserved.contains(&entity.id)
        }) {
            if specialist.deploy_remaining > 0 {
                continue;
            }
            if specialist.deployed {
                let legal_target = visible_enemy.iter().any(|enemy| {
                    let distance = specialist.pos.distance_sq(enemy.pos);
                    distance <= i64::from(spec(specialist.kind).range).pow(2)
                        && (specialist.kind != Kind::Loom
                            || distance >= i64::from(LOOM_MIN_RANGE).pow(2))
                        && (specialist.kind != Kind::Bulwark || in_front_arc(specialist, enemy.pos))
                        && self.line_of_sight(specialist.pos, enemy.pos, Some(specialist.id))
                });
                if !legal_target && self.tick.is_multiple_of(450) {
                    let _ = self.ai_issue(
                        player,
                        Command::Deploy {
                            units: vec![specialist.id],
                        },
                    );
                }
            } else {
                let legal_target = visible_enemy.iter().any(|enemy| {
                    let distance = specialist.pos.distance_sq(enemy.pos);
                    distance <= i64::from(spec(specialist.kind).range).pow(2)
                        && (specialist.kind != Kind::Loom
                            || distance >= i64::from(LOOM_MIN_RANGE).pow(2))
                        && self.line_of_sight(specialist.pos, enemy.pos, Some(specialist.id))
                });
                let near_front = specialist.pos.distance_sq(self.map.gate_pos) <= front_radius;
                if legal_target
                    || (near_front && (self.tick / 15 + u64::from(specialist.id)).is_multiple_of(7))
                {
                    if specialist.kind == Kind::Bulwark
                        && let Some(enemy) = visible_enemy.iter().find(|enemy| {
                            let distance = specialist.pos.distance_sq(enemy.pos);
                            distance <= i64::from(spec(specialist.kind).range).pow(2)
                                && self.line_of_sight(
                                    specialist.pos,
                                    enemy.pos,
                                    Some(specialist.id),
                                )
                        })
                        && !in_front_arc(specialist, enemy.pos)
                    {
                        let _ = self.ai_issue(
                            player,
                            Command::Face {
                                units: vec![specialist.id],
                                target: enemy.pos,
                            },
                        );
                    }
                    let _ = self.ai_issue(
                        player,
                        Command::Deploy {
                            units: vec![specialist.id],
                        },
                    );
                }
            }
        }
        let mut attackers: Vec<u32> = own
            .iter()
            .filter(|entity| {
                is_combat_unit(entity.kind)
                    && entity.kind != scout_kind
                    && !reserved.contains(&entity.id)
                    && at_rest(entity)
            })
            .map(|entity| entity.id)
            .collect();
        let mut surge_units: Vec<u32> = own
            .iter()
            .filter(|entity| {
                is_combat_unit(entity.kind)
                    && !reserved.contains(&entity.id)
                    && !entity.deployed
                    && entity.deploy_remaining == 0
                    && entity.surge_remaining == 0
                    && entity.surge_cooldown == 0
                    && !matches!(entity.order, Order::Capture)
                    && at_rest(entity)
            })
            .map(|entity| entity.id)
            .collect();
        attackers.sort_unstable();
        surge_units.sort_unstable();
        let surge_cost = SURGE_PRESSURE.saturating_mul(surge_units.len() as u32);
        if surge_units.len() >= 2
            && !visible_enemy.is_empty()
            && self.players[player as usize].pressure >= surge_cost.saturating_add(FLOOD_PRESSURE)
        {
            let _ = self.ai_issue(player, Command::Surge { units: surge_units });
        }
        if attackers.len() >= 2 && self.tick >= self.ai_level.first_attack() {
            let target = if self.seat_count() > 2 {
                self.ai_target(player, visible_enemy)
            } else {
                visible_enemy
                    .iter()
                    .find(|entity| entity.kind == Kind::Headquarters)
                    .map(|entity| entity.pos)
                    .unwrap_or_else(|| self.ai_fallback_target(player))
            };
            let _ = self.ai_issue(
                player,
                Command::AttackMove {
                    units: attackers,
                    target,
                    queued: false,
                },
            );
        }
    }

    /// The Compact's own machines: Pans boil where they stand on dry
    /// ground, and a Salter crusts a deep lane for the army once it has one
    /// worth sending across.
    fn ai_compact(&mut self, player: u8, own: &[Entity]) {
        let pans: Vec<u32> = own
            .iter()
            .filter(|entity| {
                entity.kind == Kind::Pan
                    && entity.build_remaining == 0
                    && !entity.deployed
                    && entity.deploy_remaining == 0
                    && matches!(entity.order, Order::Idle)
                    && entity.path.is_empty()
                    && self.may_deploy_here(entity)
            })
            .map(|entity| entity.id)
            .collect();
        if !pans.is_empty() {
            let _ = self.ai_issue(
                player,
                Command::SetDeployed {
                    units: pans,
                    deployed: true,
                },
            );
        }
        let army = own
            .iter()
            .filter(|entity| is_combat_unit(entity.kind))
            .count();
        if army < AI_TIDE_MINIMUM
            || self.tick < self.ai_level.first_attack()
            || self.players[player as usize].pressure
                < bw_content::LAY_PRESSURE_PER_ROW * bw_content::LAY_MAX_ROWS
        {
            return;
        }
        // A Salter not already laying: idle, or walking to a mouth.
        let Some(salter) = own.iter().find(|entity| {
            entity.kind == Kind::Salter
                && entity.build_remaining == 0
                && matches!(entity.order, Order::Idle | Order::Move { .. })
        }) else {
            return;
        };
        let Some(arm) = self
            .ai_arms(player)
            .into_iter()
            .find(|&arm| self.depth(Terrain::lane(arm as u8)) == Some(Depth::Deep))
        else {
            return;
        };
        let near = self.ai_near_mouth(player, arm);
        let far = self.ai_far_mouth(player, arm);
        // Start from whichever bank the Salter stands nearer.
        let (start, end) = if salter.pos.distance_sq(near) <= salter.pos.distance_sq(far) {
            (near, far)
        } else {
            (far, near)
        };
        if salter.pos.distance_sq(start) > i64::from(FP * 3).pow(2) {
            if !matches!(salter.order, Order::Idle) {
                return;
            }
            let _ = self.ai_issue(
                player,
                Command::Move {
                    units: vec![salter.id],
                    target: start,
                    queued: false,
                },
            );
            return;
        }
        // Aim at the last water before the far bank.
        let ((sx, sy), (ex, ey)) = (start.cell_xy(), end.cell_xy());
        let (dx, dy) = if (ex - sx).abs() >= (ey - sy).abs() {
            ((ex - sx).signum(), 0)
        } else {
            (0, (ey - sy).signum())
        };
        let (mut x, mut y) = (ex, ey);
        for _ in 0..4 {
            if self.map.terrain(x, y).tidal_arm().is_some() {
                break;
            }
            x -= dx;
            y -= dy;
        }
        let target = Pos::cell(x, y);
        if self.lay_plan(salter.pos, target).is_ok() {
            let _ = self.ai_issue(
                player,
                Command::Lay {
                    unit: salter.id,
                    target,
                },
            );
        }
    }

    fn ai_issue(&mut self, player: u8, command: Command) -> Result<(), String> {
        let result = self.issue(player, command);
        if result.is_ok() {
            self.events.push(Event::text(
                self.tick,
                EventKind::AiOrder,
                Some(player),
                "practice AI order",
            ));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn ids_for(world: &World, player: u8, kind: Kind) -> Vec<u32> {
        world
            .entities
            .iter()
            .filter(|entity| entity.owner == player && entity.kind == kind)
            .map(|entity| entity.id)
            .collect()
    }

    fn issue_and_step(world: &mut World, player: u8, command: Command) {
        world
            .issue(player, command)
            .expect("test command should be legal");
        world.step();
    }

    fn build_complete(world: &mut World, worker: u32, kind: Kind, pos: Pos) -> u32 {
        issue_and_step(
            world,
            0,
            Command::Build {
                worker,
                kind,
                pos,
                queued: false,
            },
        );
        let id = loop {
            if let Some(id) = world
                .entities
                .iter()
                .find(|entity| entity.owner == 0 && entity.kind == kind && entity.pos == pos)
                .map(|entity| entity.id)
            {
                break id;
            }
            world.step();
            assert!(world.tick < 10, "building command did not apply");
        };
        for _ in 0..2_000 {
            world.step();
            if world
                .entity(id)
                .is_some_and(|entity| entity.build_remaining == 0)
            {
                return id;
            }
        }
        panic!("building did not complete");
    }

    #[test]
    fn initial_fixture_is_mirrored_and_keeps_basin_separated() {
        let world = World::new(7, Faction::Union);
        assert_eq!((world.map.width, world.map.height), (128, 128));
        assert_eq!(ids_for(&world, 0, Kind::Hook).len(), 6);
        assert_eq!(ids_for(&world, 1, Kind::Wick).len(), 6);
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|entity| entity.kind == Kind::Headquarters)
                .count(),
            2
        );
        assert_eq!(
            world.map.terrain(60, 64),
            Terrain::Deep,
            "the short central bridge must not be dry land"
        );
        assert_eq!(world.map.terrain(64, 49), Terrain::Lane0);
        assert_eq!(world.map.terrain(64, 79), Terrain::Lane1);
        assert_eq!(world.map.terrain(64, 120), Terrain::Rim1);
        assert_eq!(
            world.map.terrain(64, 112),
            Terrain::Deep,
            "the lake reaches the rim"
        );
        assert!(
            world
                .map
                .wells
                .iter()
                .all(|well| world.map.terrain(well.cell_xy().0, well.cell_xy().1) != Terrain::Deep)
        );
        assert!(
            world
                .map
                .resources
                .iter()
                .all(|resource| resource.kind == ResourceKind::Salvage)
        );
        world.validate_invariants().expect("initial invariants");
    }

    #[test]
    fn identical_inputs_produce_identical_hashes() {
        let mut a = World::new(123, Faction::Union);
        let mut b = World::new(123, Faction::Union);
        let worker = ids_for(&a, 0, Kind::Hook)[0];
        let resource = a
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        let command = Command::Gather {
            units: vec![worker],
            resource,
        };
        a.issue(0, command.clone()).expect("command A");
        b.issue(0, command).expect("command B");
        for _ in 0..900 {
            a.step();
            b.step();
            assert_eq!(a.state_hash(), b.state_hash());
        }
    }

    #[test]
    fn worker_gathers_and_returns_a_load() {
        let mut world = World::new(4, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        let starting = world.players[0].salvage;
        issue_and_step(
            &mut world,
            0,
            Command::Gather {
                units: vec![worker],
                resource,
            },
        );
        // Since rules 20 the headquarters trickles salvage too, so the
        // test waits for the deposit itself.
        let mut deposited = false;
        for _ in 0..600 {
            world.step();
            if world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Deposit)
            {
                deposited = true;
                break;
            }
        }
        assert!(deposited);
        let trickle = (world.tick as u32 * HQ_SALVAGE_PER_MINUTE).div_ceil(60 * 30);
        assert!(world.players[0].salvage > starting + trickle);
    }

    #[test]
    fn a_replay_player_reproduces_the_batch_replay_and_follows_a_growing_file() {
        let dir = std::env::temp_dir().join(format!("brinewake-player-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("match.replay.json");
        let mut world = World::new(19, Faction::Union);
        world.ai_enabled = false;
        let worker = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        for _ in 0..40 {
            world.step();
        }
        world
            .issue(
                0,
                Command::Move {
                    units: vec![worker],
                    target: Pos::cell(30, 60),
                    queued: false,
                },
            )
            .unwrap();
        for _ in 0..60 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        let mut player = ReplayPlayer::open(&path).unwrap();
        while player.step().unwrap() {}
        assert_eq!(player.tick(), 100);
        assert_eq!(
            player.world().state_hash(),
            World::replay(&path).unwrap().state_hash()
        );
        assert_eq!(player.world().state_hash(), world.state_hash());
        // The match goes on and the file grows; the player takes up the rest.
        for _ in 0..50 {
            world.step();
        }
        world.export_replay(&path).unwrap();
        assert!(player.extend_from(&path).unwrap());
        while player.step().unwrap() {}
        assert_eq!(player.tick(), 150);
        assert_eq!(player.world().state_hash(), world.state_hash());
        // A revealed observer sees every cell; the hash ignores the flag.
        let mut observed = player.world().clone();
        let hash = observed.state_hash();
        observed.revealed = true;
        assert!(observed.visible(0, Pos::cell(120, 120)));
        assert!(observed.visible(1, Pos::cell(5, 5)));
        assert_eq!(observed.state_hash(), hash);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A two-seat world with a third seat added by hand: a headquarters at
    /// the north-west, no workers. The Split Basin has no third start, so this stands
    /// in for a three-seat map until one exists.
    /// A Confluence match, Union against two Assembly seats, with a
    /// Reedguard for seat 2 beside its headquarters.
    fn three_seat_world(seed: u64) -> World {
        let mut world = World::with_map(
            seed,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Assembly],
        )
        .expect("three seats fit the Confluence");
        world.ai_enabled = false;
        world.spawn_unit(2, Kind::Reedguard, THIRD_SEAT_GUARD);
        world.reset_fixture_origin().expect("clean fixture");
        world
    }

    const THIRD_SEAT_GUARD: Pos = Pos::cell(80, 150);

    fn destroy_headquarters(world: &mut World, seat: u8) {
        for entity in &mut world.entities {
            if entity.owner == seat && entity.kind == Kind::Headquarters {
                entity.hp = 0;
            }
        }
        world.entities.retain(|entity| entity.hp > 0);
    }

    #[test]
    fn a_three_seat_view_rotates_the_local_seat_to_zero() {
        let mut world = three_seat_world(5);
        world.lane_hold = vec![10, 20, 30];
        world.players[2].salvage = 333;
        for local in 0..3u8 {
            let view = world.relabeled_for(local);
            assert_eq!(
                view.players[0].salvage,
                world.players[local as usize].salvage
            );
            assert_eq!(view.lane_hold[0], world.lane_hold[local as usize]);
            for (a, b) in world.entities.iter().zip(&view.entities) {
                assert_eq!(a.id, b.id);
                assert_eq!(b.owner, (a.owner + 3 - local) % 3, "rotation, not a swap");
            }
            // The other seats keep their order after the local one.
            let order: Vec<u32> = view.lane_hold.clone();
            let expected: Vec<u32> = (0..3)
                .map(|k| world.lane_hold[((local + k) % 3) as usize])
                .collect();
            assert_eq!(order, expected);
        }
    }

    #[test]
    fn a_seat_knocked_out_earns_nothing_more() {
        // Trial 10: the Compact's pressure kept filling after its
        // headquarters fell, and spilled into salvage (rules 19).
        let mut world = three_seat_world(6);
        world.players[2].pressure = world.players[2].pressure_cap;
        destroy_headquarters(&mut world, 2);
        world.step();
        assert!(world.is_eliminated(2));
        let (pressure, salvage) = (world.players[2].pressure, world.players[2].salvage);
        let standing = world.players[0].pressure;
        for _ in 0..TICK_HZ * 60 {
            world.step();
        }
        assert_eq!(world.players[2].pressure, pressure, "no pressure");
        assert_eq!(world.players[2].salvage, salvage, "no overflow salvage");
        assert!(
            world.players[0].pressure > standing,
            "a seat still in earns"
        );
    }

    #[test]
    fn a_recycled_worker_walks_home_frees_its_crew_and_returns_half_its_salvage() {
        let mut world = World::new(31, Faction::Union);
        world.ai_enabled = false;
        let hq = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| (e.id, e.pos))
            .unwrap();
        // The starting worker farthest from the headquarters, so it walks.
        let worker = world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind == Kind::Hook)
            .max_by_key(|e| (e.pos.distance_sq(hq.1), e.id))
            .map(|e| e.id)
            .unwrap();
        let riveter = world
            .entities
            .iter()
            .find(|e| e.id == worker)
            .map(|e| e.pos)
            .map(|pos| world.spawn_unit(0, Kind::Riveter, pos))
            .unwrap();
        assert!(
            world
                .issue(
                    0,
                    Command::Recycle {
                        units: vec![riveter]
                    }
                )
                .is_err(),
            "only workers are recycled"
        );
        let (salvage, crew) = (world.players[0].salvage, world.players[0].crew);
        world
            .issue(
                0,
                Command::Recycle {
                    units: vec![worker],
                },
            )
            .expect("a worker is recycled");
        let mut recycled = None;
        for _ in 0..TICK_HZ * 30 {
            world.step();
            if let Some(event) = world
                .events
                .iter()
                .find(|e| e.kind == EventKind::Recycled && e.entity == Some(worker))
            {
                recycled = Some(event.clone());
                break;
            }
            assert!(
                !world.events.iter().any(|e| e.kind == EventKind::Death),
                "recycling is not a loss"
            );
        }
        let event = recycled.expect("the worker reaches its headquarters");
        assert_eq!(event.other, Some(hq.0));
        assert!(world.entity(worker).is_none(), "the worker is gone");
        let refund = spec(Kind::Hook).salvage * RECYCLE_REFUND_PERCENT / 100;
        assert_eq!(refund, 25);
        assert_eq!(event.amount, 25);
        assert!(world.players[0].salvage >= salvage + refund);
        assert_eq!(world.players[0].crew, crew - spec(Kind::Hook).crew);
    }

    #[test]
    fn recycling_needs_a_yard_to_walk_to() {
        let mut world = World::new(32, Faction::Union);
        world.ai_enabled = false;
        let worker = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        world
            .entities
            .retain(|e| !(e.owner == 0 && e.kind == Kind::Headquarters));
        assert!(
            world
                .issue(
                    0,
                    Command::Recycle {
                        units: vec![worker]
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn a_fallen_headquarters_knocks_a_seat_out_and_the_last_one_wins() {
        let mut world = three_seat_world(6);
        let seat_two_units = world.entities.iter().filter(|e| e.owner == 2).count();
        assert_eq!(
            seat_two_units, 8,
            "a headquarters, six workers and the guard"
        );
        destroy_headquarters(&mut world, 2);
        world.step();
        assert_eq!(world.outcome, None, "two seats still stand");
        assert!(world.is_eliminated(2));
        assert_eq!(world.eliminated, vec![(2, 0)]);
        assert!(
            world.entities.iter().all(|e| e.owner != 2),
            "its machines fall"
        );
        assert!(
            world
                .map
                .resources
                .iter()
                .any(|r| r.scrap && r.pos == THIRD_SEAT_GUARD),
            "a fallen combat machine leaves a wreck where it stood"
        );
        assert!(
            world
                .events
                .iter()
                .any(|e| e.kind == EventKind::Eliminated && e.player == Some(2))
        );
        assert!(
            world.issue(2, Command::Stop { units: vec![] }).is_err(),
            "no orders from an eliminated seat"
        );
        assert_eq!(world.standing_seats(), vec![0, 1]);
        destroy_headquarters(&mut world, 1);
        world.step();
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
        assert!(
            !world.is_eliminated(1),
            "the match ended; only seats knocked out mid-match are recorded"
        );
    }

    #[test]
    fn a_rotated_view_reads_the_map_through_its_own_seat_numbers() {
        let world = three_seat_world(12);
        for local in 0..3u8 {
            let view = world.relabeled_for(local);
            for seat in 0..3u8 {
                let canonical = (seat + local) % 3;
                assert_eq!(view.layout_seat(seat), canonical);
                assert_eq!(view.arms_of(seat), world.arms_of(canonical));
                assert_eq!(view.start_of(seat), world.start_of(canonical));
            }
            for arm in 0..3 {
                let banks = world.arm_banks(arm).map(|bank| (bank + 3 - local) % 3);
                assert_eq!(view.arm_banks(arm), banks);
            }
            assert_eq!(
                view.own_mouth(0, view.arms_of(0)[0]),
                world.own_mouth(local, world.arms_of(local)[0])
            );
        }
        assert_eq!(world.relabeled_for(1).relabeled_for(2).view_turn, 0);
    }

    #[test]
    fn a_three_seat_surrender_knocks_out_only_that_seat() {
        let mut world = three_seat_world(7);
        world.issue(1, Command::Surrender).expect("surrender");
        for _ in 0..8 {
            world.step();
        }
        assert_eq!(world.outcome, None);
        assert!(world.is_eliminated(1));
        world.issue(2, Command::Surrender).expect("surrender");
        for _ in 0..8 {
            world.step();
        }
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
        let state = world.state_hash();
        assert_eq!(
            state,
            world.clone().state_hash(),
            "an elimination hashes the same twice"
        );
    }

    #[test]
    fn an_arm_is_written_as_the_split_basins_boolean_or_as_its_number() {
        for (arm, written) in [(Arm(0), "true"), (Arm(1), "false"), (Arm(2), "2")] {
            let json = serde_json::to_string(&arm).expect("serialize");
            assert_eq!(json, written);
            assert_eq!(serde_json::from_str::<Arm>(&json).expect("parse"), arm);
        }
        let command = Command::SetTide { arm: Arm::NORTH };
        assert_eq!(
            serde_json::to_string(&command).expect("serialize"),
            r#"{"SetTide":{"north":true}}"#,
            "a two-arm command keeps its old bytes"
        );
    }

    #[test]
    fn the_confluence_tide_dries_one_arm_and_a_switch_walks_the_arms_in_turn() {
        let mut world = three_seat_world(31);
        world.gate.owner = Some(0);
        world.players[0].pressure = 1_000;
        let lane = |world: &World, arm: u8| world.depth(Terrain::lane(arm));
        assert_eq!(
            lane(&world, 2),
            Some(Depth::Shallow),
            "every arm starts shallow"
        );
        issue_and_step(&mut world, 0, Command::SetTide { arm: Arm(2) });
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
        }
        assert_eq!(world.gate.tide, Tide::Open);
        assert_eq!(world.gate.dry_arm, Arm(2));
        assert_eq!(lane(&world, 2), Some(Depth::Dry));
        assert_eq!(lane(&world, 0), Some(Depth::Deep));
        assert_eq!(world.depth(Terrain::rim(1)), Some(Depth::Deep));
        assert!(
            world
                .events
                .iter()
                .any(|e| e.kind == EventKind::GateChanged && e.text == "S open")
        );
        assert!(
            world.issue(0, Command::SetTide { arm: Arm(3) }).is_err(),
            "the Confluence has three arms"
        );
        for expected in [Arm(0), Arm(1), Arm(2)] {
            world.gate.locked_until = 0;
            issue_and_step(&mut world, 0, Command::SwitchGate);
            for _ in 0..GATE_WARNING_TICKS {
                world.step();
            }
            assert_eq!(world.gate.dry_arm, expected, "a switch opens the next arm");
        }
        world.validate_invariants().expect("invariants");
    }

    #[test]
    fn a_confluence_hold_needs_both_lanes_that_touch_the_seat() {
        let mut world = three_seat_world(32);
        world.gate.owner = Some(0);
        let layout = world.map.layout();
        assert_eq!(layout.arms_of(0).collect::<Vec<_>>(), vec![0, 1]);
        let [e_mouth, _] = layout.arm_mouths(0);
        let [w_mouth, _] = layout.arm_mouths(1);
        world.spawn_for_tests(0, Kind::Riveter, e_mouth);
        world.step();
        assert!(world.holds_lane(0, 0));
        assert_eq!(world.lane_hold[0], 0, "one of two lanes counts for nothing");
        world.spawn_for_tests(0, Kind::Riveter, w_mouth);
        world.step();
        assert!(world.holds_lane(0, 1));
        assert_eq!(world.lane_hold[0], 1, "both lanes held: the gauge runs");
        assert!(
            !world.holds_lane(1, 0),
            "a seat without the station holds nothing"
        );
    }

    #[test]
    fn every_seat_on_every_map_can_raise_the_practice_ais_base() {
        for map in MapId::ALL {
            let layout = map.layout();
            let factions = vec![Faction::Assembly; layout.seat_count()];
            let world = World::with_map(3, map, &factions).expect("world");
            for seat in world.seats() {
                let sites = &layout.seats[usize::from(seat)].ai;
                let site = |(x, y): (i32, i32)| Pos::cell(x, y);
                assert!(
                    world.can_place(Kind::Works, site(sites.works)),
                    "{map:?} {seat} works"
                );
                assert!(
                    world.can_place(Kind::Dropoff, site(sites.dropoff)),
                    "{map:?} {seat} dropoff"
                );
                assert!(
                    world.map.wells.contains(&site(sites.condenser)),
                    "{map:?} {seat} condenser on a well"
                );
                assert!(
                    world.ai_drydock_site(seat, Faction::Assembly).is_some(),
                    "{map:?} {seat} has a Drydock site that launches a Barge"
                );
            }
        }
    }

    #[test]
    fn a_confluence_match_saves_loads_and_replays_to_the_same_hash() {
        let mut world = three_seat_world(33);
        // The practice AI plays seat 1 as it does on the Split Basin.
        world.ai_enabled = true;
        let worker = world
            .entities
            .iter()
            .find(|e| e.owner == 2 && e.kind.is_worker())
            .map(|e| e.id)
            .expect("a worker");
        world
            .issue(
                2,
                Command::Move {
                    units: vec![worker],
                    target: Pos::cell(88, 120),
                    queued: false,
                },
            )
            .expect("move");
        for _ in 0..1_800 {
            world.step();
        }
        world.validate_invariants().expect("invariants");
        let root = std::env::temp_dir();
        let save = root.join(format!("bw-confluence-save-{}.json", std::process::id()));
        let replay = root.join(format!("bw-confluence-replay-{}.json", std::process::id()));
        world.save(&save).expect("save");
        let loaded = World::load(&save).expect("load");
        assert_eq!(loaded.map.id, MapId::Confluence);
        assert_eq!(loaded.state_hash(), world.state_hash());
        world.export_replay(&replay).expect("export");
        let replayed = World::replay(&replay).expect("replay");
        assert_eq!(replayed.state_hash(), world.state_hash());
        let _ = std::fs::remove_file(save);
        let _ = std::fs::remove_file(replay);
    }

    #[test]
    fn a_three_seat_ai_breaks_a_count_then_goes_for_the_weakest_it_can_reach() {
        let mut world = three_seat_world(42);
        // Nobody counting, every arm shallow: both opponents are in reach and
        // look equally weak, so it takes the first.
        assert_eq!(world.ai_target(2, &[]), Pos::cell(45, 45));
        // Seat 1 counts: seat 2 stands at its own mouth of the S arm.
        world.gate.owner = Some(1);
        world.lane_hold[1] = 5;
        assert_eq!(world.ai_target(2, &[]), Pos::cell(94, 111));
        // E dry, W and S deep: seat 2 can reach no one, so it waits at its
        // mouth toward the weakest.
        world.lane_hold[1] = 0;
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm(0);
        assert_eq!(world.ai_target(2, &[]), Pos::cell(70, 105));
        // S dry: seat 1 is in reach, and seat 0 is not.
        world.gate.dry_arm = Arm(2);
        assert_eq!(world.ai_target(2, &[]), Pos::cell(147, 73));
    }

    #[test]
    fn the_practice_ai_plays_both_other_seats_on_the_confluence() {
        let mut world = World::with_map(
            41,
            MapId::Confluence,
            &[Faction::Union, Faction::Union, Faction::Assembly],
        )
        .expect("world");
        let mut fielded = [0usize; 3];
        for _ in 0..9_000 {
            world.step();
            for event in &world.events {
                if event.kind == EventKind::ProductionCompleted
                    && let Some(seat) = event.player
                    && event
                        .entity
                        .and_then(|id| world.entity(id))
                        .is_some_and(|unit| is_combat_unit(unit.kind))
                {
                    fielded[usize::from(seat)] += 1;
                }
            }
            if world.outcome.is_some() {
                break;
            }
        }
        world.validate_invariants().expect("invariants");
        for seat in [1u8, 2] {
            let built = |kind: Kind| {
                world
                    .entities
                    .iter()
                    .any(|e| e.owner == seat && e.kind == kind && e.build_remaining == 0)
            };
            assert!(built(Kind::Works), "seat {seat} raised a Works");
            assert!(built(Kind::Drydock), "seat {seat} raised a Drydock");
            let issued = world
                .command_log
                .iter()
                .filter(|record| record.player == seat && record.accepted)
                .count();
            // Machines built, not standing: the two AI seats now fight over
            // the station, and one may have lost its first wave by now.
            let army = fielded[usize::from(seat)];
            assert!(issued > 20, "seat {seat} gave {issued} orders");
            assert!(army >= 3, "seat {seat} built {army} fighting machines");
        }
        assert!(
            world.command_log.iter().all(|record| record.player != 0),
            "the person's seat is left alone"
        );
    }

    #[test]
    fn the_practice_ai_takes_the_confluence_station_and_starts_a_count() {
        // Two AI seats used to channel side by side on the station for a
        // whole match, and a garrison on the far bank of a shared arm
        // denied every hold.
        let mut world = World::with_map(
            17,
            MapId::Confluence,
            &[Faction::Union, Faction::Union, Faction::Assembly],
        )
        .expect("world");
        let mut captured = None;
        while world.tick < 18_000 && world.outcome.is_none() {
            world.step();
            if captured.is_none()
                && world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::GateCaptured)
            {
                captured = Some(world.tick);
            }
            if world.lane_hold.iter().any(|&hold| hold > 0) {
                break;
            }
        }
        assert!(captured.is_some(), "an AI seat took the station");
        assert!(
            world.lane_hold.iter().any(|&hold| hold > 0) || world.outcome.is_some(),
            "a hold count started by tick {}",
            world.tick
        );
        world.validate_invariants().expect("invariants");
    }

    #[test]
    fn a_compact_practice_ai_boils_pans_and_wins() {
        // The Compact against a seat that does nothing, on both maps.
        for map in [MapId::SplitBasin, MapId::Confluence] {
            let factions: &[Faction] = if map == MapId::Confluence {
                &[Faction::Union, Faction::Compact, Faction::Assembly]
            } else {
                &[Faction::Union, Faction::Compact]
            };
            let mut world = World::with_map(31, map, factions).expect("world");
            let (mut pans, mut rows, mut beams) = (0, 0, 0);
            while world.outcome.is_none() && world.tick < 60_000 && !world.is_eliminated(0) {
                world.step();
                pans = pans.max(
                    world
                        .entities
                        .iter()
                        .filter(|e| e.owner == 1 && e.kind == Kind::Pan && e.deployed)
                        .count(),
                );
                for event in &world.events {
                    match event.kind {
                        EventKind::Laid if event.player == Some(1) => rows += 1,
                        EventKind::Shot
                            if event
                                .entity
                                .and_then(|id| world.entity(id))
                                .is_some_and(|e| e.kind == Kind::Heliostat) =>
                        {
                            beams += 1
                        }
                        _ => {}
                    }
                }
            }
            eprintln!(
                "{map:?}: tick {} outcome {:?} out0 {} pans {pans} rows {rows} beams {beams}",
                world.tick,
                world.outcome,
                world.is_eliminated(0)
            );
            assert!(pans >= 1, "the Compact AI boils a Pan on {map:?}");
            // On the Split Basin it wins; on the Confluence another AI may
            // win first, but the idle seat never outlasts them.
            if map == MapId::SplitBasin {
                assert_eq!(world.outcome, Some(Outcome::Victory(1)));
            } else {
                assert!(world.outcome.is_some() || world.is_eliminated(0));
            }
        }
    }

    #[test]
    fn a_compact_practice_ai_crusts_a_deep_lane_for_its_army() {
        let mut world =
            World::with_factions(31, &[Faction::Union, Faction::Compact]).expect("world");
        world.tick = world.ai_level.first_attack();
        world.ai_last_tick = world.tick;
        world.gate.tide = Tide::Flood;
        world.players[1].pressure = 300;
        world.players[1].cap = 200;
        let arm = world.ai_arms(1)[0];
        let mouth = world.ai_near_mouth(1, arm).cell_xy();
        for i in 0..4 {
            let id = world.spawn_unit(1, Kind::Brander, Pos::cell(mouth.0 + 3, mouth.1 + i));
            world.entity_mut(id).unwrap().order = Order::Hold;
        }
        let salter = world.spawn_unit(1, Kind::Salter, Pos::cell(mouth.0 + 6, mouth.1));
        world.entity_mut(salter).unwrap().order = Order::Idle;
        let mut rows = 0;
        for _ in 0..(40 * TICK_HZ) {
            world.step();
            rows += world
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Laid && e.player == Some(1))
                .count();
        }
        assert!(
            rows >= 20,
            "the Salter crusted {rows} rows across the deep lane"
        );
    }

    #[test]
    fn an_easy_practice_ai_builds_less_and_replays_as_itself() {
        // Against a seat that does nothing: Easy runs fewer workers, holds
        // its army home for eight minutes and so wins later.
        let play = |level: AiLevel| {
            let mut world = World::new(29, Faction::Union);
            world.ai_level = level;
            let mut workers = 0;
            while world.outcome.is_none() && world.tick < 40_000 {
                world.step();
                if world.tick == 9_000 {
                    workers = world
                        .entities
                        .iter()
                        .filter(|e| e.owner == 1 && e.kind.is_worker())
                        .count();
                }
            }
            (workers, world)
        };
        let (normal_workers, normal) = play(AiLevel::Normal);
        let (easy_workers, world) = play(AiLevel::Easy);
        assert!(
            easy_workers < normal_workers,
            "{easy_workers} workers against {normal_workers}"
        );
        assert_eq!(world.outcome, Some(Outcome::Victory(1)));
        assert!(
            world.tick >= normal.tick + 60 * TICK_HZ,
            "easy won at {}, normal at {}",
            world.tick,
            normal.tick
        );
        // The level is part of the match: saved, hashed and replayed.
        let dir = std::env::temp_dir().join(format!(
            "bw-easy-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        world
            .export_replay(dir.join("easy.replay.json"))
            .expect("export");
        let replayed = World::replay(dir.join("easy.replay.json")).expect("replay");
        assert_eq!(replayed.ai_level, AiLevel::Easy);
        assert_eq!(replayed.state_hash(), world.state_hash());
        world.save(dir.join("easy.json")).expect("save");
        let loaded = World::load(dir.join("easy.json")).expect("load");
        assert_eq!(loaded.ai_level, AiLevel::Easy);
        assert_eq!(loaded.state_hash(), world.state_hash());
        let _ = std::fs::remove_dir_all(&dir);
        // Normal is never written, so older saves and replays read the same.
        let normal = World::new(29, Faction::Union);
        let json = serde_json::to_string(&normal).expect("serialize");
        assert!(!json.contains("ai_level"));
    }

    #[test]
    fn a_two_seat_world_never_records_an_elimination() {
        let mut world = World::new(8, Faction::Union);
        world.ai_enabled = false;
        destroy_headquarters(&mut world, 1);
        world.step();
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
        assert!(world.eliminated.is_empty());
        let json = serde_json::to_string(&world).expect("serialize");
        assert!(
            !json.contains("eliminated"),
            "two-seat saves keep their old shape"
        );
    }

    #[test]
    fn with_factions_allows_a_mirror_match_and_refuses_three_on_the_split_basin() {
        let mirror =
            World::with_factions(9, &[Faction::Assembly, Faction::Assembly]).expect("mirror");
        assert_eq!(mirror.players[0].faction, mirror.players[1].faction);
        assert!(
            World::with_factions(9, &[Faction::Union, Faction::Assembly, Faction::Union]).is_err()
        );
    }

    #[test]
    fn a_relabelled_view_swaps_every_seat_and_keeps_ids_and_positions() {
        let mut world = World::new(23, Faction::Union);
        world.ai_enabled = false;
        for _ in 0..40 {
            world.step();
        }
        let view = world.relabeled_for(1);
        assert_eq!(view.tick, world.tick);
        assert_eq!(view.players[0].faction, world.players[1].faction);
        for (a, b) in world.entities.iter().zip(&view.entities) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.pos, b.pos);
            assert_eq!(a.owner ^ 1, b.owner);
        }
        for y in (0..128).step_by(7) {
            for x in (0..128).step_by(7) {
                let pos = Pos::cell(x, y);
                assert_eq!(view.visible(0, pos), world.visible(1, pos));
                assert_eq!(view.visible(1, pos), world.visible(0, pos));
            }
        }
        assert_eq!(
            view.relabeled_for(1),
            world,
            "relabelling twice is the identity"
        );
        assert_eq!(world.relabeled_for(0), world);

        // Every per-player field, given two values that can be told apart, so
        // that a field nobody swapped fails here. Relabelling twice is an
        // involution whether or not a field is swapped, so that check alone
        // proves nothing about completeness.
        let mut marked = world.clone();
        marked.lane_hold = vec![11, 29];
        marked.gate.owner = Some(0);
        marked.gate.capture_player = Some(1);
        marked.beacons = vec![
            Beacon {
                owner: 0,
                pos: Pos::cell(10, 10),
                until: 100,
                radius: SOUND_RADIUS,
                ray_to: None,
            },
            Beacon {
                owner: 1,
                pos: Pos::cell(20, 20),
                until: 200,
                radius: SOUND_RADIUS,
                ray_to: None,
            },
        ];
        marked.enemy_reports = vec![vec![(1, 2, 3)], vec![(4, 5, 6)]];
        marked.players[0].salvage = 111;
        marked.players[1].salvage = 222;
        marked.outcome = Some(Outcome::Victory(0));
        let swapped = marked.relabeled_for(1);
        assert_eq!(swapped.lane_hold, [29, 11], "the hold gauge swaps");
        assert_eq!(swapped.gate.owner, Some(1), "the station's owner swaps");
        assert_eq!(
            swapped.gate.capture_player,
            Some(0),
            "who is capturing swaps"
        );
        assert_eq!(
            swapped.beacons.iter().map(|b| b.owner).collect::<Vec<_>>(),
            vec![1, 0],
            "SOUND pings swap"
        );
        assert_eq!(
            swapped.enemy_reports,
            [vec![(4, 5, 6)], vec![(1, 2, 3)]],
            "enemy reports swap"
        );
        assert_eq!(swapped.players[0].salvage, 222, "player slots swap");
        assert_eq!(swapped.players[1].salvage, 111, "player slots swap");
        assert_eq!(
            swapped.outcome,
            Some(Outcome::Victory(1)),
            "the winner swaps"
        );
        assert_eq!(
            swapped.observations[0].len(),
            world.observations[1].len(),
            "each seat reads the other's knowledge"
        );
        assert!(
            swapped
                .command_log
                .iter()
                .zip(&world.command_log)
                .all(|(a, b)| a.player == b.player ^ 1),
            "the command log swaps its seats"
        );
        // A command built against the view validates against the canonical
        // world for the guest's canonical seat.
        let worker = view
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .expect("guest worker in the view");
        let command = Command::Move {
            units: vec![worker],
            target: Pos::cell(100, 60),
            queued: false,
        };
        assert!(world.validate(1, &command).is_ok());
        assert!(world.validate(0, &command).is_err());
    }

    #[test]
    fn idle_machines_and_defence_nests_engage_enemies_on_their_own() {
        let mut world = World::new(41, Faction::Union);
        world.ai_enabled = false;
        let riveter = world.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        let enemy = riveter + 1;
        let tower = riveter + 2;
        let mut template = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .cloned()
            .expect("worker template");
        template.order = Order::Idle;
        template.path.clear();
        template.carried = 0;
        template.carried_kind = None;
        // An idle Riveter with an enemy worker in sight but out of range.
        let mut r = template.clone();
        r.id = riveter;
        r.kind = Kind::Riveter;
        r.hp = spec(Kind::Riveter).health;
        r.max_hp = r.hp;
        r.pos = Pos::cell(30, 30);
        let mut e = template.clone();
        e.id = enemy;
        e.owner = 1;
        e.pos = Pos::cell(36, 30);
        // A finished defence nest with an enemy inside its new eight-cell reach.
        let mut t = template.clone();
        t.id = tower;
        t.kind = Kind::Tower;
        t.hp = spec(Kind::Tower).health;
        t.max_hp = t.hp;
        t.pos = Pos::cell(30, 60);
        let mut victim = template.clone();
        victim.id = tower + 1;
        victim.owner = 1;
        victim.pos = Pos::cell(37, 60);
        let victim_hp = victim.hp;
        world.entities.extend([r, e, t, victim]);
        world.step();
        assert!(
            matches!(
                world.entity(riveter).unwrap().order,
                Order::AttackMove { .. }
            ),
            "an idle machine closes on a visible enemy"
        );
        for _ in 0..240 {
            world.step();
        }
        assert!(
            world
                .entity(enemy)
                .is_none_or(|e| e.hp < spec(Kind::Hook).health),
            "the pursuing machine fires without an order"
        );
        assert!(
            world.entity(tower + 1).is_none_or(|e| e.hp < victim_hp),
            "the idle nest fires at eight cells"
        );
        assert_eq!(spec(Kind::Tower).range, 8 * 256);
    }

    #[test]
    fn an_interrupted_site_resumes_when_a_worker_is_sent_back() {
        let mut world = World::new(31, Faction::Union);
        world.ai_enabled = false;
        let worker = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .expect("starting worker")
            .id;
        let site_pos = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| Pos::cell(e.pos.cell_xy().0 + 6, e.pos.cell_xy().1 + 6))
            .expect("site beside the headquarters");
        issue_and_step(
            &mut world,
            0,
            Command::Build {
                worker,
                kind: Kind::Dropoff,
                pos: site_pos,
                queued: false,
            },
        );
        let site = world
            .entities
            .iter()
            .find(|e| e.kind == Kind::Dropoff)
            .expect("site spawned")
            .id;
        for _ in 0..240 {
            world.step();
        }
        let remaining = world.entity(site).expect("site").build_remaining;
        assert!(remaining > 0, "the site is still under construction");
        // Pull the builder away: the site stalls.
        issue_and_step(
            &mut world,
            0,
            Command::Stop {
                units: vec![worker],
            },
        );
        for _ in 0..60 {
            world.step();
        }
        let stalled = world.entity(site).expect("site").build_remaining;
        assert!(
            stalled >= remaining.saturating_sub(2),
            "a stopped builder makes no progress"
        );
        // Sending the worker back resumes the work instead of repairing hull.
        let salvage = world.players[0].salvage;
        issue_and_step(
            &mut world,
            0,
            Command::Repair {
                units: vec![worker],
                target: site,
            },
        );
        assert!(matches!(
            world.entity(worker).expect("worker").order,
            Order::Build { target } if target == site
        ));
        for _ in 0..3000 {
            world.step();
            if world.entity(site).is_some_and(|e| e.build_remaining == 0) {
                break;
            }
        }
        assert_eq!(world.entity(site).expect("site").build_remaining, 0);
        assert!(
            world.players[0].salvage >= salvage,
            "resuming spends no salvage; gathering may add some"
        );
    }

    #[test]
    fn build_repair_train_rally_and_cancel_are_functional() {
        let mut world = World::new(5, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let dropoff = build_complete(&mut world, worker, Kind::Dropoff, Pos::cell(25, 60));
        if let Some(entity) = world.entity_mut(dropoff) {
            entity.hp = entity.max_hp / 2;
        }
        issue_and_step(
            &mut world,
            0,
            Command::Repair {
                units: vec![worker],
                target: dropoff,
            },
        );
        for _ in 0..500 {
            world.step();
            if world
                .entity(dropoff)
                .is_some_and(|entity| entity.hp == entity.max_hp)
            {
                break;
            }
        }
        assert!(
            world
                .entity(dropoff)
                .is_some_and(|entity| entity.hp == entity.max_hp)
        );

        world.players[0].salvage = world.players[0].salvage.max(spec(Kind::Works).salvage);
        let works = build_complete(&mut world, worker, Kind::Works, Pos::cell(25, 66));
        issue_and_step(
            &mut world,
            0,
            Command::Rally {
                building: works,
                pos: Pos::cell(30, 64),
            },
        );
        world.players[0].salvage = world.players[0].salvage.max(spec(Kind::Riveter).salvage);
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: works,
                kind: Kind::Riveter,
            },
        );
        for _ in 0..700 {
            world.step();
            if ids_for(&world, 0, Kind::Riveter).len() == 1 {
                break;
            }
        }
        let riveters = ids_for(&world, 0, Kind::Riveter);
        assert_eq!(riveters.len(), 1);
        assert_eq!(
            world.entity(riveters[0]).map(|entity| entity.order.clone()),
            Some(Order::Move {
                target: Pos::cell(30, 64)
            })
        );

        world.players[0].salvage = world.players[0].salvage.max(spec(Kind::Sounder).salvage);
        world.players[0].pressure = world.players[0].pressure.max(spec(Kind::Sounder).pressure);
        let before = world.players[0].salvage;
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: works,
                kind: Kind::Sounder,
            },
        );
        assert!(world.players[0].salvage < before);
        issue_and_step(&mut world, 0, Command::Cancel { building: works });
        assert!(world.players[0].salvage > before.saturating_sub(spec(Kind::Sounder).salvage));
    }

    #[test]
    fn capture_switch_warning_and_lock_are_exact_seed_durations() {
        let mut world = World::new(6, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let works = build_complete(&mut world, worker, Kind::Works, Pos::cell(25, 60));
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: works,
                kind: Kind::Sounder,
            },
        );
        for _ in 0..700 {
            world.step();
            if ids_for(&world, 0, Kind::Sounder).len() == 1 {
                break;
            }
        }
        let sounder = ids_for(&world, 0, Kind::Sounder)[0];
        let gate_pos = world.map.gate_pos;
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![sounder],
                target: gate_pos,
                queued: false,
            },
        );
        for _ in 0..500 {
            world.step();
            if world.entity(sounder).is_some_and(|entity| {
                entity.pos.distance_sq(world.map.gate_pos) <= i64::from(FP * 2).pow(2)
            }) {
                break;
            }
        }
        assert!(world.entity(sounder).is_some_and(|entity| {
            entity.pos.distance_sq(world.map.gate_pos) <= i64::from(FP * 2).pow(2)
        }));
        issue_and_step(
            &mut world,
            0,
            Command::Capture {
                units: vec![sounder],
            },
        );
        // The first capture of the neutral station takes half as long again
        // (rules 15).
        let work = world.capture_work();
        assert_eq!(work, CAPTURE_TICKS * 3 / 2);
        for _ in 0..work.saturating_sub(2) {
            world.step();
        }
        assert_ne!(world.gate.owner, Some(0));
        world.step();
        assert_eq!(world.gate.owner, Some(0));

        world.issue(0, Command::SwitchGate).expect("switch command");
        world.step();
        let warning_until = world.gate.warning_until.expect("warning scheduled");
        assert_eq!(
            warning_until - world.tick,
            u64::from(GATE_WARNING_TICKS - 1)
        );
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
        }
        assert!(world.gate.warning_until.is_none());
        // From the neutral tide the switch opens the north.
        assert_eq!(world.gate.tide, Tide::Open);
        assert!(world.gate.north_dry());
        assert_eq!(
            world.gate.locked_until,
            world.tick - 1 + u64::from(GATE_LOCK_TICKS)
        );
        assert!(world.issue(0, Command::SwitchGate).is_err());
    }

    #[test]
    fn brace_and_pack_use_a_serialized_one_second_timer() {
        let mut world = World::new(82, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let works = build_complete(&mut world, worker, Kind::Works, Pos::cell(25, 60));
        world.players[0].salvage = world.players[0].salvage.max(spec(Kind::Bulwark).salvage);
        world.players[0].pressure = world.players[0].pressure.max(spec(Kind::Bulwark).pressure);
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: works,
                kind: Kind::Bulwark,
            },
        );
        for _ in 0..800 {
            world.step();
            if ids_for(&world, 0, Kind::Bulwark).len() == 1 {
                break;
            }
        }
        let bulwark = ids_for(&world, 0, Kind::Bulwark)[0];
        issue_and_step(
            &mut world,
            0,
            Command::Deploy {
                units: vec![bulwark],
            },
        );
        assert!(world.entity(bulwark).is_some_and(|entity| {
            entity.deploy_remaining == TICK_HZ as u32 - 1 && !entity.deployed
        }));
        for _ in 0..(TICK_HZ - 1) {
            world.step();
        }
        assert!(
            world
                .entity(bulwark)
                .is_some_and(|entity| { entity.deploy_remaining == 0 && entity.deployed })
        );
        issue_and_step(
            &mut world,
            0,
            Command::Deploy {
                units: vec![bulwark],
            },
        );
        assert!(world.entity(bulwark).is_some_and(|entity| {
            entity.deploy_remaining == TICK_HZ as u32 - 1 && entity.deployed
        }));
        for _ in 0..(TICK_HZ - 1) {
            world.step();
        }
        assert!(
            world
                .entity(bulwark)
                .is_some_and(|entity| { entity.deploy_remaining == 0 && !entity.deployed })
        );
    }

    #[test]
    fn enemy_combat_presence_pauses_gate_capture() {
        let mut world = World::new(83, Faction::Union);
        world.ai_enabled = false;
        let gate = world.map.gate_pos;
        let capturer = world.spawn_unit(0, Kind::Sounder, gate);
        let blocker = world.spawn_unit(1, Kind::Reedguard, gate);
        issue_and_step(
            &mut world,
            0,
            Command::Capture {
                units: vec![capturer],
            },
        );
        for _ in 0..200 {
            world.step();
        }
        assert_eq!(world.gate.owner, None);
        assert_eq!(world.gate.capture_progress, 0);
        if let Some(entity) = world.entity_mut(blocker) {
            entity.pos = Pos::cell(90, 90);
            entity.order = Order::Idle;
        }
        for _ in 0..world.capture_work() {
            world.step();
        }
        assert_eq!(world.gate.owner, Some(0));
    }

    #[test]
    fn an_enemy_four_cells_out_still_contests_a_capture() {
        let mut world = World::new(83, Faction::Union);
        world.ai_enabled = false;
        let gate = world.map.gate_pos;
        let capturer = world.spawn_unit(0, Kind::Sounder, gate);
        let blocker = world.spawn_unit(
            1,
            Kind::Bulwark,
            Pos {
                x: gate.x + FP * 3,
                y: gate.y,
            },
        );
        if let Some(entity) = world.entity_mut(blocker) {
            entity.order = Order::Hold;
            entity.attack_cooldown = u32::MAX / 2;
        }
        issue_and_step(
            &mut world,
            0,
            Command::Capture {
                units: vec![capturer],
            },
        );
        for _ in 0..60 {
            world.step();
        }
        assert_eq!(world.gate.capture_progress, 0, "three cells out contests");
        if let Some(entity) = world.entity_mut(blocker) {
            entity.pos = Pos {
                x: gate.x + FP * 6,
                y: gate.y,
            };
        }
        for _ in 0..30 {
            world.step();
        }
        assert!(world.gate.capture_progress > 0, "six cells out does not");
    }

    #[test]
    fn hidden_entities_cannot_be_targeted_and_gate_is_public() {
        let world = World::new(8, Faction::Union);
        let enemy = ids_for(&world, 1, Kind::Wick)[0];
        let attacker = ids_for(&world, 0, Kind::Riveter);
        assert!(!world.entity_visible(0, enemy));
        assert!(
            world
                .clone()
                .issue(
                    0,
                    Command::Attack {
                        units: attacker,
                        target: enemy,
                    }
                )
                .is_err()
        );
        assert!(world.visible(0, world.map.gate_pos));
        let knowledge = world.knowledge(0);
        assert!(knowledge.visible.iter().all(|entity| entity.owner != 0));
        assert!(knowledge.own.iter().all(|entity| entity.owner == 0));
    }

    #[test]
    fn fog_uses_last_seen_building_position_and_never_current_hidden_position() {
        let mut world = World::new(81, Faction::Union);
        world.ai_enabled = false;
        let enemy_hq = ids_for(&world, 1, Kind::Headquarters)[0];
        let observed_pos = Pos::cell(20, 64);
        if let Some(entity) = world.entity_mut(enemy_hq) {
            entity.pos = observed_pos;
        }
        world.update_visibility_memory();
        assert!(
            world
                .knowledge(0)
                .visible
                .iter()
                .any(|entity| entity.id == enemy_hq)
        );
        let hidden_pos = Pos::cell(115, 20);
        if let Some(entity) = world.entity_mut(enemy_hq) {
            entity.pos = hidden_pos;
        }
        world.update_visibility_memory();
        let knowledge = world.knowledge(0);
        let stale = knowledge
            .stale
            .iter()
            .find(|entity| entity.id == enemy_hq)
            .expect("previously visible building remains stale");
        assert_eq!(stale.pos, observed_pos);
        assert_ne!(stale.pos, hidden_pos);
        assert!(
            knowledge
                .map
                .resources
                .iter()
                .any(|resource| resource.remaining == 0)
        );
    }

    #[test]
    fn save_load_resume_and_replay_preserve_state_hash() {
        let mut world = World::new(9, Faction::Union);
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let resource = world.map.resources[0].id;
        world
            .issue(
                0,
                Command::Gather {
                    units: vec![worker],
                    resource,
                },
            )
            .expect("gather");
        for _ in 0..80 {
            world.step();
        }
        let root = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let save_path = root.join(format!("brinewake-sim-{}-{stamp}.json", std::process::id()));
        let replay_path = root.join(format!(
            "brinewake-sim-{}-{stamp}.replay",
            std::process::id()
        ));
        world.save(&save_path).expect("save");
        let mut loaded = World::load(&save_path).expect("load");
        assert_eq!(world.state_hash(), loaded.state_hash());
        for _ in 0..240 {
            world.step();
            loaded.step();
            assert_eq!(world.state_hash(), loaded.state_hash());
        }
        world.export_replay(&replay_path).expect("replay export");
        let replayed = World::replay(&replay_path).expect("replay");
        assert_eq!(world.state_hash(), replayed.state_hash());
        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(replay_path);
    }

    #[test]
    fn ai_issues_only_normal_commands_and_stays_bounded() {
        let mut world = World::new(11, Faction::Union);
        for _ in 0..900 {
            world.step();
        }
        assert!(world.command_log.iter().any(|record| record.player == 1));
        assert!(world.command_log.len() < 2_000);
        assert!(world.command_log.iter().all(|record| {
            record.player != 1
                || !matches!(record.command, Command::Attack { .. })
                || record.accepted
        }));
        world.validate_invariants().expect("AI invariants");
    }

    /// The practice AI now runs a whole game off its own commands: an
    /// economy, a Drydock, the tier-two roles and the sluice.  What it wins
    /// with is asserted in the tests below; this one holds the line that it
    /// gets there through the normal command path at all.
    #[test]
    fn practice_ai_can_carry_a_match_through_to_a_win() {
        let mut world = World::new(12, Faction::Union);
        for _ in 0..15_000 {
            world.step();
            if world.outcome.is_some() {
                break;
            }
        }
        assert!(
            world.command_log.iter().any(|record| {
                record.player == 1
                    && matches!(
                        record.command,
                        Command::Build { .. } | Command::Train { .. }
                    )
                    && record.accepted
            }),
            "AI never built an economy or queued a unit"
        );
        assert!(
            world.command_log.iter().any(|record| {
                record.player == 1
                    && matches!(record.command, Command::AttackMove { .. })
                    && record.accepted
            }),
            "AI never advanced a force through the normal command path"
        );
        assert_eq!(world.outcome, Some(Outcome::Victory(1)));
        // The recording plays back to the same state: its AI's machines
        // muster at the door as they did live.
        let path = std::env::temp_dir().join(format!("bw-ai-replay-{}.json", std::process::id()));
        world.export_replay(&path).expect("export");
        let replayed = World::replay(&path).expect("replay");
        let _ = std::fs::remove_file(&path);
        assert_eq!(replayed.state_hash(), world.state_hash());
    }

    #[test]
    fn the_practice_ai_builds_a_drydock_and_fields_every_tier_two_role() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut world = World::new(12, faction);
            let mut drydock = false;
            let mut seen: Vec<Kind> = Vec::new();
            for _ in 0..15_000 {
                world.step();
                for entity in world.entities.iter().filter(|e| e.owner == 1 && e.hp > 0) {
                    drydock |= entity.kind == Kind::Drydock && entity.build_remaining == 0;
                    if !seen.contains(&entity.kind) {
                        seen.push(entity.kind);
                    }
                }
                if world.outcome.is_some() {
                    break;
                }
            }
            let ai = world.players[1].faction;
            assert!(drydock, "{ai:?}: the practice AI never finished a Drydock");
            for role in ai.drydock_roles() {
                assert!(
                    seen.contains(&role),
                    "{ai:?}: the practice AI never fielded a {}",
                    role.name()
                );
            }
        }
    }

    #[test]
    fn the_practice_ai_takes_the_sluice_and_runs_its_count_without_freezing_it() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut world = World::new(12, faction);
            let mut best = 0;
            for _ in 0..15_000 {
                world.step();
                best = best.max(world.lane_hold[1]);
                if world.outcome.is_some() {
                    break;
                }
            }
            let ai = world.players[1].faction;
            assert_eq!(
                world.gate.owner,
                Some(1),
                "{ai:?}: the practice AI never took the sluice"
            );
            // Rules 20: a flood freezes every count, its own too, so against
            // an idle side it has no reason to drown the lanes.
            assert!(
                !world.command_log.iter().any(|record| {
                    record.player == 1 && matches!(record.command, Command::Flood)
                }),
                "{ai:?}: the practice AI froze its own count with a flood"
            );
            assert_eq!(world.outcome, Some(Outcome::Victory(1)));
            // Rules 20 (seed 12): the AI playing the Union, its count 83 s
            // in, broke the idle headquarters seven seconds before the
            // tide; the one playing the Assembly won on the tide.  Either
            // way its count ran most of the way unbroken.
            let tide = world.events.iter().any(|event| {
                event.kind == EventKind::Victory && event.text.contains("HOLDS THE TIDE")
            });
            assert!(
                tide || best >= TIDE_HOLD_TICKS * 3 / 4,
                "{ai:?}: the practice AI won, but its count got only {best}"
            );
        }
    }

    #[test]
    fn the_practice_ai_ferries_a_landing_party_in_its_transport() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut world = World::new(12, faction);
            let mut carried = false;
            for _ in 0..15_000 {
                // Against an idle opponent the AI can win on the tide
                // before its transport is ever worth using (it did once
                // machines walked diagonals); keep the gauge empty so the
                // match lasts long enough to see the ferry.
                world.lane_hold = vec![0; world.players.len()];
                world.step();
                carried |= world
                    .entities
                    .iter()
                    .any(|entity| entity.owner == 1 && entity.hp > 0 && entity.aboard.is_some());
                if world.outcome.is_some() {
                    break;
                }
            }
            let ai = world.players[1].faction;
            let accepted = |wanted: fn(&Command) -> bool| {
                world
                    .command_log
                    .iter()
                    .any(|record| record.player == 1 && record.accepted && wanted(&record.command))
            };
            assert!(
                accepted(|command| matches!(command, Command::Board { .. })),
                "{ai:?}: the practice AI never loaded its transport"
            );
            assert!(carried, "{ai:?}: nothing ever rode the transport");
            assert!(
                accepted(|command| matches!(command, Command::Unload { .. })),
                "{ai:?}: the practice AI never set its hold down"
            );
        }
    }

    #[test]
    fn the_practice_ai_dries_the_lane_its_force_is_waiting_at() {
        // The sluice is the practice AI's, the far side is open and its
        // own lane is under deep water with a force stood at the mouth.
        let mut world = World::new(41, Faction::Assembly);
        world.ai_enabled = false;
        world.step();
        world.gate.owner = Some(1);
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm::from_north(false);
        world.gate.locked_until = 0;
        world.players[1].pressure = 250;
        let mouth = world.ai_mouths(1)[0];
        let (x, y) = mouth.cell_xy();
        for offset in 0..3 {
            place(&mut world, 1, Kind::Riveter, x + 1 + offset, y + 2);
        }
        world.ai_enabled = true;
        for _ in 0..40 {
            world.step();
        }
        assert!(
            world.command_log.iter().any(|record| {
                record.player == 1
                    && record.accepted
                    && matches!(record.command, Command::SetTide { arm: Arm::NORTH })
            }),
            "the practice AI left its force stood at a drowned lane"
        );
    }

    #[test]
    fn the_practice_ai_turns_banked_pressure_into_salvage() {
        let mut world = World::new(12, Faction::Union);
        for _ in 0..12_000 {
            world.step();
            if world.outcome.is_some() {
                break;
            }
        }
        assert!(
            world.command_log.iter().any(|record| {
                record.player == 1
                    && record.accepted
                    && matches!(record.command, Command::Reclaim { .. })
            }),
            "the practice AI sat on a full pressure gauge"
        );
    }

    fn place(world: &mut World, owner: u8, kind: Kind, x: i32, y: i32) -> u32 {
        world.spawn_for_tests(owner, kind, Pos::cell(x, y))
    }

    #[test]
    fn caps_grow_with_the_base_and_income_follows_the_sluice_and_valves() {
        let mut world = World::new(31, Faction::Union);
        world.ai_enabled = false;
        world.step();
        assert_eq!(world.players[0].cap, CREW_CAP_BASE);
        assert_eq!(world.players[0].pressure_cap, PRESSURE_CAP_BASE);
        place(&mut world, 0, Kind::Works, 24, 50);
        place(&mut world, 0, Kind::Dropoff, 24, 54);
        place(&mut world, 0, Kind::Drydock, 28, 50);
        place(&mut world, 0, Kind::Condenser, 19, 69);
        world.step();
        assert_eq!(
            world.players[0].cap,
            CREW_CAP_BASE + CREW_CAP_WORKS + CREW_CAP_YARD + CREW_CAP_DRYDOCK
        );
        assert_eq!(
            world.players[0].pressure_cap,
            PRESSURE_CAP_BASE + PRESSURE_CAP_CONDENSER
        );
        world.players[0].upgrades = vec![Upgrade::Overpressure];
        world.step();
        assert_eq!(
            world.players[0].pressure_cap,
            PRESSURE_CAP_BASE + PRESSURE_CAP_CONDENSER + PRESSURE_CAP_OVERPRESSURE
        );
        // Income over a minute: base 60, one condenser 120, the sluice 15,
        // Bleed Valves 60.
        world.players[0].pressure = 0;
        world.players[0].pressure_remainder = 0;
        world.gate.owner = Some(0);
        world.players[0].upgrades = vec![Upgrade::BleedValves, Upgrade::Overpressure];
        for _ in 0..1800 {
            world.step();
        }
        assert_eq!(world.players[0].pressure, 60 + 120 + 15 + 60);
    }

    #[test]
    fn a_lane_wreck_waits_for_the_tide_and_a_dredger_takes_it_wet() {
        let mut world = World::new(32, Faction::Union);
        world.ai_enabled = false;
        // With the north open the north wreck is dry and the south wreck
        // lies under deep water.
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm::from_north(true);
        let north = world
            .map
            .resources
            .iter()
            .find(|r| r.pos == Pos::cell(54, 49))
            .unwrap()
            .clone();
        let south = world
            .map
            .resources
            .iter()
            .find(|r| r.pos == Pos::cell(54, 79))
            .unwrap()
            .clone();
        assert!(world.gatherable(Kind::Hook, &north));
        assert!(!world.gatherable(Kind::Hook, &south));
        assert!(world.gatherable(Kind::Dredger, &south));
        let worker = place(&mut world, 0, Kind::Hook, 53, 80);
        let dredger = place(&mut world, 0, Kind::Dredger, 55, 80);
        issue_and_step(
            &mut world,
            0,
            Command::Gather {
                units: vec![worker, dredger],
                resource: south.id,
            },
        );
        for _ in 0..400 {
            world.step();
        }
        let hook = world.entity(worker).unwrap();
        let dredge = world.entity(dredger).unwrap();
        assert_eq!(hook.carried, 0, "the Hook waits at a drowned wreck");
        assert!(
            matches!(hook.order, Order::Gather { .. }),
            "and keeps its order"
        );
        assert!(
            dredge.carried > 0
                || world
                    .entity(dredger)
                    .unwrap()
                    .pos
                    .distance_sq(Pos::cell(54, 79))
                    > i64::from(FP * 3).pow(2),
            "the Dredger gathers it or is already hauling"
        );
        assert!(world.speed_per_tick(dredge) >= 0);
    }

    #[test]
    fn capture_scales_with_machines_and_the_switch_costs_pressure_and_can_be_cancelled() {
        let mut world = World::new(33, Faction::Union);
        world.ai_enabled = false;
        let gate = world.map.gate_pos;
        let (gx, gy) = gate.cell_xy();
        let mut ids = Vec::new();
        for dx in 0..3 {
            ids.push(place(&mut world, 0, Kind::Riveter, gx + dx, gy + 1));
        }
        ids.sort_unstable();
        issue_and_step(&mut world, 0, Command::Capture { units: ids.clone() });
        let mut captured_at = None;
        for _ in 0..40 * 30 {
            world.step();
            if world.gate.owner == Some(0) {
                captured_at = Some(world.tick);
                break;
            }
        }
        let captured_at = captured_at.expect("captured");
        assert!(
            (15 * 30..25 * 30).contains(&captured_at),
            "three machines take twenty seconds on the neutral station, took {captured_at}"
        );
        // The switch costs pressure and an enemy beside the station cancels it.
        world.players[0].pressure = SWITCH_PRESSURE - 1;
        assert!(world.issue(0, Command::SwitchGate).is_err());
        world.players[0].pressure = 100;
        issue_and_step(&mut world, 0, Command::SwitchGate);
        assert_eq!(world.players[0].pressure, 60);
        assert!(world.gate.warning_until.is_some());
        let before_cancel = world.players[0].pressure;
        place(&mut world, 1, Kind::Reedguard, gx - 1, gy);
        world.step();
        assert!(
            world.gate.warning_until.is_none(),
            "an enemy at the station cancels the switch"
        );
        let cancelled = world
            .events
            .iter()
            .find(|e| e.kind == EventKind::SwitchCancelled)
            .expect("the cancel is announced");
        // Rules 13: the cancel gives the price back and says so.
        assert_eq!(cancelled.amount, SWITCH_PRESSURE as i32);
        assert!(
            cancelled.text.contains("40 pressure back"),
            "{}",
            cancelled.text
        );
        assert!(
            world.players[0].pressure >= before_cancel + SWITCH_PRESSURE,
            "the price is refunded, {} from {before_cancel}",
            world.players[0].pressure
        );
        // With the enemy still beside the station a new switch is refused up
        // front, with the reason, and costs nothing.
        let pressure = world.players[0].pressure;
        assert_eq!(
            world.issue(0, Command::SwitchGate),
            Err("an enemy stands at the station".to_string())
        );
        assert_eq!(
            world.issue(0, Command::SetTide { arm: Arm::SOUTH }),
            Err("an enemy stands at the station".to_string())
        );
        assert!(world.station_contested_for(0));
        world.step();
        assert!(world.players[0].pressure >= pressure);
        assert!(world.gate.warning_until.is_none());
    }

    #[test]
    fn wading_hurts_the_keel_holds_and_the_tide_can_be_won() {
        let mut world = World::new(34, Faction::Union);
        world.ai_enabled = false;
        // With the north open, a machine in the shallow... no: the south is
        // deep then.  Test the shallow case at the neutral tide first, then
        // the dry north once opened.
        let wet = place(&mut world, 1, Kind::Reedguard, 60, 79);
        let riveter = place(&mut world, 0, Kind::Riveter, 40, 60);
        assert_eq!(
            world.modified_damage(riveter, wet, 12),
            15,
            "shallow at the neutral tide"
        );
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm::from_north(true);
        let dry = place(&mut world, 1, Kind::Reedguard, 60, 49);
        let wet = place(&mut world, 1, Kind::Reedguard, 60, 79);
        assert_eq!(world.modified_damage(riveter, dry, 12), 12);
        assert_eq!(
            world.modified_damage(riveter, wet, 12),
            18,
            "swamped in the deep south"
        );
        // The keel: a headquarters with a Works standing stops at a quarter.
        let hq = ids_for(&world, 1, Kind::Headquarters)[0];
        let max = world.entity(hq).unwrap().max_hp;
        let works = place(&mut world, 1, Kind::Works, 100, 50);
        assert!(!world.deal_damage(hq, max));
        assert_eq!(world.entity(hq).unwrap().hp, max / 4);
        if let Some(entity) = world.entity_mut(works) {
            entity.hp = 0;
        }
        world.entities.retain(|entity| entity.hp > 0);
        assert!(world.deal_damage(hq, max));
        // The tide: combat machines at both crossing mouths for ninety seconds.
        let mut world = World::new(35, Faction::Union);
        world.ai_enabled = false;
        world.gate.owner = Some(0);
        for mouths in CROSSING_MOUTHS {
            let (x, y) = mouths[0].cell_xy();
            place(&mut world, 0, Kind::Riveter, x, y);
        }
        for _ in 0..(TIDE_HOLD_TICKS as usize + 5) {
            world.step();
            if world.outcome.is_some() {
                break;
            }
        }
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
        assert!(
            world
                .events
                .iter()
                .any(|e| e.kind == EventKind::Victory && e.text.contains("HOLDS THE TIDE"))
        );
    }

    #[test]
    fn upgrades_run_one_at_a_time_change_the_field_and_refund_on_cancel() {
        let mut world = World::new(36, Faction::Union);
        world.ai_enabled = false;
        world.players[0].salvage = 2_000;
        world.players[0].pressure = 300;
        world.players[0].pressure_cap = 300;
        let works = place(&mut world, 0, Kind::Works, 24, 50);
        let bulwark = place(&mut world, 0, Kind::Bulwark, 30, 60);
        let riveter = place(&mut world, 0, Kind::Riveter, 31, 60);
        let enemy_works = place(&mut world, 1, Kind::Works, 100, 50);
        issue_and_step(
            &mut world,
            0,
            Command::Upgrade {
                building: works,
                upgrade: Upgrade::Plate,
            },
        );
        assert!(world.entity(works).unwrap().upgrade.is_some());
        // Rules 15: a second and third upgrade queue behind the first, paid
        // now; a cancel takes the last queued one back whole.
        let paid = world.players[0].salvage;
        issue_and_step(
            &mut world,
            0,
            Command::Upgrade {
                building: works,
                upgrade: Upgrade::Siege,
            },
        );
        issue_and_step(
            &mut world,
            0,
            Command::Upgrade {
                building: works,
                upgrade: Upgrade::Temper,
            },
        );
        assert_eq!(
            world.entity(works).unwrap().upgrade_queue,
            vec![Upgrade::Siege, Upgrade::Temper]
        );
        assert_eq!(world.players[0].salvage, paid - 120 - 300);
        assert!(
            world
                .issue(
                    0,
                    Command::Upgrade {
                        building: works,
                        upgrade: Upgrade::Siege,
                    }
                )
                .is_err(),
            "an upgrade is queued once"
        );
        issue_and_step(&mut world, 0, Command::CancelUpgrade { building: works });
        issue_and_step(&mut world, 0, Command::CancelUpgrade { building: works });
        assert!(world.entity(works).unwrap().upgrade_queue.is_empty());
        assert_eq!(world.players[0].salvage, paid);
        assert!(world.entity(works).unwrap().upgrade.is_some());
        let before = world.players[0].salvage;
        issue_and_step(&mut world, 0, Command::CancelUpgrade { building: works });
        assert!(world.entity(works).unwrap().upgrade.is_none());
        assert_eq!(world.players[0].salvage, before + 120 * 3 / 4);
        issue_and_step(
            &mut world,
            0,
            Command::Upgrade {
                building: works,
                upgrade: Upgrade::Plate,
            },
        );
        for _ in 0..(40 * 30 + 2) {
            world.step();
        }
        assert!(world.players[0].upgrades.contains(&Upgrade::Plate));
        assert_eq!(
            world.entity(bulwark).unwrap().max_hp,
            spec(Kind::Bulwark).health * 125 / 100
        );
        assert_eq!(world.modified_damage(riveter, enemy_works, 12), 18);
        world.players[0].upgrades.push(Upgrade::Siege);
        assert_eq!(world.modified_damage(riveter, enemy_works, 12), 24);
        world.players[0].upgrades.push(Upgrade::Cranes);
        assert_eq!(world.carry_for(0, Kind::Hook), 7);
        assert_eq!(world.carry_for(0, Kind::Dredger), 10);
    }

    #[test]
    fn vent_sound_repair_scouts_caissons_reports_and_scrap_work() {
        let mut world = World::new(37, Faction::Union);
        world.ai_enabled = false;
        world.players[0].pressure = 300;
        world.players[0].pressure_cap = 300;
        for worker in ids_for(&world, 0, Kind::Hook) {
            world.clear_order(worker);
        }
        let hq = ids_for(&world, 0, Kind::Headquarters)[0];
        let riveter = place(&mut world, 0, Kind::Riveter, 30, 60);
        let base = world.attack_cooldown(world.entity(riveter).unwrap());
        issue_and_step(&mut world, 0, Command::Vent { building: hq });
        assert!(world.players[0].vent_remaining > 0);
        let vented = world.attack_cooldown(world.entity(riveter).unwrap());
        assert!(vented < base && vented >= base * 7 / 10);
        // SOUND lights a far cell for five seconds.
        let sounder = place(&mut world, 0, Kind::Sounder, 40, 30);
        let far = Pos::cell(45, 30);
        let _ = far;
        let probe = Pos::cell(40, 30 + 5);
        issue_and_step(&mut world, 0, Command::Sound { unit: sounder });
        assert!(world.beacons.len() == 1);
        assert!(world.visible(0, probe));
        for _ in 0..(SOUND_TICKS as usize + 1) {
            world.step();
        }
        assert!(world.beacons.is_empty());
        // A Caulker mends a damaged Riveter for salvage.
        let caulker = place(&mut world, 0, Kind::Caulker, 31, 60);
        if let Some(entity) = world.entity_mut(riveter) {
            entity.hp = 50;
        }
        let salvage = world.players[0].salvage;
        for _ in 0..60 {
            world.step();
        }
        let mended = world.entity(riveter).unwrap().hp;
        assert!(mended >= 50 + 8, "five hull a second, got {mended}");
        assert!(world.players[0].salvage < salvage, "mending spends salvage");
        let _ = caulker;
        // A scout on Hold sees twice as far.
        let scout = place(&mut world, 0, Kind::Tidewatch, 20, 20);
        let sight = world.sight_of(world.entity(scout).unwrap());
        issue_and_step(&mut world, 0, Command::Hold { units: vec![scout] });
        assert_eq!(world.sight_of(world.entity(scout).unwrap()), sight * 2);
        // A deployed Caisson blocks its cell.
        let caisson = place(&mut world, 0, Kind::Caisson, 60, 49);
        assert!(world.cell_walkable_for(Kind::Hook, (60, 49)));
        issue_and_step(
            &mut world,
            0,
            Command::Deploy {
                units: vec![caisson],
            },
        );
        for _ in 0..31 {
            world.step();
        }
        assert!(world.entity(caisson).unwrap().deployed);
        assert!(!world.cell_walkable_for(Kind::Hook, (60, 49)));
        // An enemy seen raises one report per region per twenty seconds.
        let enemy = place(&mut world, 1, Kind::Reedguard, 32, 60);
        world.step();
        let seen: Vec<&Event> = world
            .events
            .iter()
            .filter(|e| e.kind == EventKind::EnemySeen && e.player == Some(0))
            .collect();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].amount, 1);
        world.step();
        assert!(
            !world
                .events
                .iter()
                .any(|e| e.kind == EventKind::EnemySeen && e.player == Some(0))
        );
        // Scrap recovery: a killed enemy leaves a wreck worth half its cost.
        world.players[0].upgrades.push(Upgrade::ScrapRecovery);
        let wrecks = world.map.resources.len();
        if let Some(entity) = world.entity_mut(enemy) {
            entity.hp = 1;
        }
        for _ in 0..60 {
            world.step();
            if world.entity(enemy).is_none() {
                break;
            }
        }
        assert!(
            world.entity(enemy).is_none(),
            "the Riveter kills the Reedguard"
        );
        assert_eq!(world.map.resources.len(), wrecks + 1);
        assert_eq!(
            world.map.resources.last().unwrap().remaining,
            spec(Kind::Reedguard).salvage * SCRAP_UPGRADED_PERCENT / 100
        );
    }

    #[test]
    fn the_drydock_trains_tier_two_and_a_palisade_blocks() {
        let mut world = World::new(38, Faction::Assembly);
        world.ai_enabled = false;
        world.players[0].salvage = 2_000;
        world.players[0].pressure = 300;
        world.players[0].pressure_cap = 300;
        let works = place(&mut world, 0, Kind::Works, 96, 50);
        assert!(
            world
                .issue(
                    0,
                    Command::Train {
                        building: works,
                        kind: Kind::Lampwright,
                    }
                )
                .is_err(),
            "the Works does not train tier two"
        );
        let drydock = place(&mut world, 0, Kind::Drydock, 100, 50);
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: drydock,
                kind: Kind::Lampwright,
            },
        );
        for _ in 0..400 {
            world.step();
            if !ids_for(&world, 0, Kind::Lampwright).is_empty() {
                break;
            }
        }
        assert_eq!(ids_for(&world, 0, Kind::Lampwright).len(), 1);
        let worker = ids_for(&world, 0, Kind::Wick)[0];
        assert!(world.cell_walkable_for(Kind::Hook, (100, 60)));
        issue_and_step(
            &mut world,
            0,
            Command::Build {
                worker,
                kind: Kind::Palisade,
                pos: Pos::cell(100, 60),
                queued: false,
            },
        );
        assert!(!world.cell_walkable_for(Kind::Hook, (100, 60)));
    }

    #[test]
    fn a_long_move_from_either_base_reaches_the_sluice() {
        for (seed, owner, start) in [(41, 0u8, Pos::cell(24, 60)), (42, 1u8, Pos::cell(104, 60))] {
            let mut world = World::new(seed, Faction::Union);
            world.ai_enabled = false;
            let mut ids = Vec::new();
            for i in 0..4 {
                let (x, y) = start.cell_xy();
                ids.push(world.spawn_for_tests(
                    owner,
                    Kind::Reedguard,
                    Pos::cell(x + i % 2, y + i / 2),
                ));
            }
            ids.sort_unstable();
            let gate = world.map.gate_pos;
            world
                .issue(
                    owner,
                    Command::Move {
                        units: ids.clone(),
                        target: gate,
                        queued: false,
                    },
                )
                .expect("move accepted");
            for _ in 0..1800 {
                world.step();
            }
            let rejected: Vec<String> = world
                .command_log
                .iter()
                .filter(|r| r.player == owner && r.applied == Some(false))
                .filter_map(|r| r.reason.clone())
                .collect();
            assert!(rejected.is_empty(), "owner {owner}: {rejected:?}");
            for id in &ids {
                let e = world.entity(*id).unwrap();
                let moved = e.pos.distance_sq(start) > i64::from(FP * 10).pow(2);
                assert!(
                    moved,
                    "owner {owner}: machine {id} stayed at {:?} with order {:?}, path {} blocked {}",
                    e.pos.cell_xy(),
                    e.order,
                    e.path.len(),
                    e.blocked_ticks
                );
            }
        }
    }

    #[test]
    fn a_group_with_a_deployed_specialist_still_moves_and_water_targets_reach_the_shore() {
        let mut world = World::new(43, Faction::Union);
        world.ai_enabled = false;
        let bulwark = world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        let riveter = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(31, 60));
        if let Some(entity) = world.entity_mut(bulwark) {
            entity.deployed = true;
        }
        let mut ids = vec![bulwark, riveter];
        ids.sort_unstable();
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: ids,
                target: Pos::cell(40, 60),
                queued: false,
            },
        );
        assert!(matches!(
            world.entity(riveter).unwrap().order,
            Order::Move { .. }
        ));
        assert!(!matches!(
            world.entity(bulwark).unwrap().order,
            Order::Move { .. }
        ));
        // A target in deep water resolves to the nearest ground.
        let sounder = world.spawn_for_tests(0, Kind::Sounder, Pos::cell(40, 60));
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![sounder],
                target: Pos::cell(56, 30),
                queued: false,
            },
        );
        for _ in 0..600 {
            world.step();
        }
        let pos = world.entity(sounder).unwrap().pos.cell_xy();
        assert!(
            pos.0 > 44,
            "the Sounder walked toward the water's edge, at {pos:?}"
        );
        // A machine walled in stops with a word instead of standing on an order.
        let hook = world.spawn_for_tests(0, Kind::Hook, Pos::cell(10, 10));
        for (x, y) in [
            (9, 9),
            (10, 9),
            (11, 9),
            (9, 10),
            (11, 10),
            (9, 11),
            (10, 11),
            (11, 11),
        ] {
            world.map.tiles[y as usize * MAP_SIZE as usize + x as usize] = Terrain::Rock;
        }
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![hook],
                target: Pos::cell(20, 20),
                queued: false,
            },
        );
        world.step();
        assert!(
            matches!(world.entity(hook).unwrap().order, Order::Idle),
            "no path: the order clears"
        );
    }

    #[test]
    fn new_workers_gather_on_their_own_and_a_rally_on_a_wreck_directs_them() {
        let mut world = World::new(7, Faction::Union);
        world.ai_enabled = false;
        let hq = ids_for(&world, 0, Kind::Headquarters)[0];
        let hq_pos = world.entity(hq).unwrap().pos;
        let nearest = world
            .nearest_salvage(0, Kind::Hook, hq_pos, AUTO_GATHER_RADIUS, true)
            .unwrap();
        let before = ids_for(&world, 0, Kind::Hook);
        world.players[0].salvage = 500;
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: hq,
                kind: Kind::Hook,
            },
        );
        for _ in 0..700 {
            world.step();
            if ids_for(&world, 0, Kind::Hook).len() > before.len() {
                break;
            }
        }
        let fresh = ids_for(&world, 0, Kind::Hook)
            .into_iter()
            .find(|id| !before.contains(id))
            .expect("a new worker");
        assert_eq!(
            world.entity(fresh).map(|e| e.order.clone()),
            Some(Order::Gather { resource: nearest }),
            "a new worker with no rally joins the nearest wreck"
        );

        // A rally beside a farther wreck sends the next worker there instead.
        let far = world
            .map
            .resources
            .iter()
            .filter(|r| r.id != nearest && r.pos.cell_xy().0 < 52 && r.remaining > 0)
            .max_by_key(|r| (r.pos.distance_sq(hq_pos), r.id))
            .unwrap()
            .clone();
        let (fx, fy) = far.pos.cell_xy();
        issue_and_step(
            &mut world,
            0,
            Command::Rally {
                building: hq,
                pos: Pos::cell(fx + 1, fy),
            },
        );
        let before = ids_for(&world, 0, Kind::Hook);
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: hq,
                kind: Kind::Hook,
            },
        );
        for _ in 0..700 {
            world.step();
            if ids_for(&world, 0, Kind::Hook).len() > before.len() {
                break;
            }
        }
        let fresh = ids_for(&world, 0, Kind::Hook)
            .into_iter()
            .find(|id| !before.contains(id))
            .expect("a second worker");
        assert_eq!(
            world.entity(fresh).map(|e| e.order.clone()),
            Some(Order::Gather { resource: far.id })
        );

        // A rally on open ground is still a move for a worker.
        issue_and_step(
            &mut world,
            0,
            Command::Rally {
                building: hq,
                pos: Pos::cell(30, 50),
            },
        );
        let before = ids_for(&world, 0, Kind::Hook);
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: hq,
                kind: Kind::Hook,
            },
        );
        for _ in 0..700 {
            world.step();
            if ids_for(&world, 0, Kind::Hook).len() > before.len() {
                break;
            }
        }
        let fresh = ids_for(&world, 0, Kind::Hook)
            .into_iter()
            .find(|id| !before.contains(id))
            .expect("a third worker");
        assert_eq!(
            world.entity(fresh).map(|e| e.order.clone()),
            Some(Order::Move {
                target: Pos::cell(30, 50)
            })
        );
    }

    #[test]
    fn a_worker_whose_wreck_runs_dry_moves_to_the_next_wreck_with_salvage() {
        let mut world = World::new(8, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let Order::Gather { resource } = world.entity(worker).unwrap().order.clone() else {
            panic!("starting workers gather");
        };
        for item in world.map.resources.iter_mut() {
            if item.id == resource {
                item.remaining = 3;
            }
        }
        for other in ids_for(&world, 0, Kind::Hook) {
            if other != worker {
                world.clear_order(other);
            }
        }
        let starting = world.players[0].salvage;
        for _ in 0..1500 {
            world.step();
            if world.players[0].salvage > starting
                && world
                    .entity(worker)
                    .is_some_and(|e| e.order != Order::Gather { resource })
            {
                break;
            }
        }
        assert!(
            world.players[0].salvage > starting,
            "the last load was delivered"
        );
        let next = world.entity(worker).map(|e| e.order.clone());
        assert!(
            matches!(next, Some(Order::Gather { resource: r }) if r != resource),
            "the worker went on to another wreck, got {next:?}"
        );
    }

    #[test]
    fn the_result_event_names_the_winner_and_siege_roles_hit_buildings_harder() {
        let mut world = World::new(9, Faction::Union);
        world.ai_enabled = false;
        let riveter = world.spawn_unit(0, Kind::Riveter, Pos::cell(40, 60));
        let sounder = world.spawn_unit(0, Kind::Sounder, Pos::cell(41, 60));
        let enemy_hq = ids_for(&world, 1, Kind::Headquarters)[0];
        let enemy_worker = ids_for(&world, 1, Kind::Wick)[0];
        assert_eq!(world.modified_damage(riveter, enemy_hq, 12), 18);
        assert_eq!(world.modified_damage(riveter, enemy_worker, 12), 12);
        assert_eq!(world.modified_damage(sounder, enemy_hq, 7), 7);
        let shot = ArtilleryShot {
            owner: 1,
            source: 0,
            from: Pos::cell(60, 60),
            target: Pos::cell(40, 60),
            impact_tick: 0,
        };
        let hq = world
            .entity(ids_for(&world, 0, Kind::Headquarters)[0])
            .unwrap()
            .clone();
        let machine = world.entity(riveter).unwrap().clone();
        assert_eq!(world.artillery_damage(&shot, &hq), 64);
        assert_eq!(world.artillery_damage(&shot, &machine), 32);

        if let Some(entity) = world.entity_mut(enemy_hq) {
            entity.hp = 0;
        }
        world.entities.retain(|entity| entity.hp > 0);
        world.step();
        let result = world
            .events
            .iter()
            .find(|event| event.kind == EventKind::Victory)
            .expect("a result event");
        assert_eq!(result.text, "BREAKWATER UNION WINS");
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
    }

    #[test]
    fn a_move_packs_a_deployed_specialist_first_and_then_sends_it() {
        let mut world = World::new(83, Faction::Union);
        world.ai_enabled = false;
        let bulwark = world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        let riveter = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(31, 60));
        issue_and_step(
            &mut world,
            0,
            Command::Deploy {
                units: vec![bulwark],
            },
        );
        for _ in 0..TICK_HZ {
            world.step();
        }
        assert!(world.entity(bulwark).is_some_and(|e| e.deployed));
        let start = world.entity(bulwark).map(|e| e.pos).unwrap();
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![bulwark, riveter],
                target: Pos::cell(44, 60),
                queued: false,
            },
        );
        let packing = world.entity(bulwark).unwrap();
        assert!(
            !packing.deploy_target && packing.deploy_remaining > 0,
            "the move starts the pack"
        );
        assert!(
            matches!(packing.after_pack, Some(Order::Move { .. })),
            "the move waits on the pack"
        );
        assert!(
            matches!(world.entity(riveter).unwrap().order, Order::Move { .. }),
            "the rest of the group goes at once"
        );
        for _ in 0..TICK_HZ {
            world.step();
        }
        let packed = world.entity(bulwark).unwrap();
        assert!(!packed.deployed && packed.after_pack.is_none());
        assert!(matches!(packed.order, Order::Move { .. }));
        for _ in 0..120 {
            world.step();
        }
        assert!(
            world.entity(bulwark).unwrap().pos.x > start.x + FP,
            "the packed machine walks to the target"
        );
    }

    #[test]
    fn stop_during_the_pack_drops_the_pending_move() {
        let mut world = World::new(84, Faction::Union);
        world.ai_enabled = false;
        let bulwark = world.spawn_for_tests(0, Kind::Bulwark, Pos::cell(30, 60));
        issue_and_step(
            &mut world,
            0,
            Command::Deploy {
                units: vec![bulwark],
            },
        );
        for _ in 0..TICK_HZ {
            world.step();
        }
        let start = world.entity(bulwark).map(|e| e.pos).unwrap();
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![bulwark],
                target: Pos::cell(44, 60),
                queued: false,
            },
        );
        issue_and_step(
            &mut world,
            0,
            Command::Stop {
                units: vec![bulwark],
            },
        );
        for _ in 0..(TICK_HZ + 120) {
            world.step();
        }
        let e = world.entity(bulwark).unwrap();
        assert!(!e.deployed && e.after_pack.is_none());
        assert_eq!(e.pos, start, "a stopped pack goes nowhere");
    }

    #[test]
    fn the_tide_starts_neutral_and_the_rims_follow_the_lanes() {
        let mut world = World::new(90, Faction::Union);
        world.ai_enabled = false;
        assert_eq!(world.gate.tide, Tide::Neutral);
        assert_eq!(world.map.terrain(60, 5), Terrain::Rim0);
        assert_eq!(world.map.terrain(60, 120), Terrain::Rim1);
        for (x, y) in [(60, 49), (60, 79), (60, 5), (60, 120)] {
            assert_eq!(
                world.depth_at(x, y),
                Some(Depth::Shallow),
                "({x},{y}) shallow at the start"
            );
            assert!(world.cell_walkable_for(Kind::Riveter, (x, y)));
        }
        // A machine on the north rim wades like one in a shallow lane.
        let on_rim = place(&mut world, 1, Kind::Reedguard, 60, 5);
        let riveter = place(&mut world, 0, Kind::Riveter, 40, 60);
        assert_eq!(world.modified_damage(riveter, on_rim, 12), 15);
        // Opening the south dries the south lane and rim and deepens the north.
        world.gate.owner = Some(0);
        world.players[0].pressure = 200;
        issue_and_step(&mut world, 0, Command::SetTide { arm: Arm::SOUTH });
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
        }
        assert_eq!(world.gate.tide, Tide::Open);
        assert!(!world.gate.north_dry());
        assert_eq!(world.depth_at(60, 79), Some(Depth::Dry));
        assert_eq!(world.depth_at(60, 120), Some(Depth::Dry));
        assert_eq!(world.depth_at(60, 49), Some(Depth::Deep));
        assert_eq!(world.depth_at(60, 5), Some(Depth::Deep));
        assert!(
            !world.cell_walkable_for(Kind::Riveter, (60, 49)),
            "deep water is a wall"
        );
        assert!(
            world.cell_walkable_for(Kind::Dredger, (60, 49)),
            "the Dredger takes it"
        );
        assert!(
            world
                .issue(0, Command::SetTide { arm: Arm::SOUTH })
                .is_err(),
            "already open"
        );
        // The holder sees the rims as it sees the lanes.
        assert!(world.visible(0, Pos::cell(60, 5)));
        assert!(world.visible(0, Pos::cell(60, 120)));
    }

    #[test]
    fn a_flood_walls_everything_swamps_the_caught_and_falls_back() {
        let mut world = World::new(91, Faction::Union);
        world.ai_enabled = false;
        world.gate.owner = Some(0);
        world.players[0].pressure = 300;
        issue_and_step(&mut world, 0, Command::SetTide { arm: Arm::NORTH });
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
        }
        world.gate.locked_until = 0;
        let caught = place(&mut world, 1, Kind::Reedguard, 58, 49);
        let start = world.entity(caught).unwrap().pos;
        assert!(world.issue(0, Command::Flood).is_ok());
        world.step();
        assert!(world.gate.flood_pending);
        for _ in 0..FLOOD_WARNING_TICKS {
            world.step();
        }
        assert_eq!(world.gate.tide, Tide::Flood);
        for (x, y) in [(60, 49), (60, 79), (60, 5), (60, 120)] {
            assert_eq!(world.depth_at(x, y), Some(Depth::Deep));
        }
        assert!(
            world
                .issue(0, Command::SetTide { arm: Arm::SOUTH })
                .is_err(),
            "no switch in a flood"
        );
        let swamped = world.entity(caught).unwrap();
        assert!(world.swamped(swamped));
        assert!(
            matches!(swamped.order, Order::Move { .. }),
            "the tide sends it to the shore"
        );
        assert!(
            world
                .events
                .iter()
                .any(|e| e.kind == EventKind::Swamped && e.entity == Some(caught))
        );
        for _ in 0..240 {
            world.step();
        }
        let out = world.entity(caught).unwrap();
        assert!(!world.swamped(out), "it waded out");
        assert_ne!(out.pos, start);
        assert!(world.cell_walkable_for(Kind::Reedguard, out.pos.cell_xy()));
        for _ in 0..FLOOD_TICKS {
            world.step();
        }
        assert_eq!(
            world.gate.tide,
            Tide::Open,
            "the flood falls back to the open side"
        );
        assert!(world.gate.north_dry());
        assert!(
            world.tick < world.gate.locked_until,
            "and the station locks"
        );
    }

    #[test]
    fn a_barge_needs_water_carries_four_and_sinks_with_its_hold() {
        let mut world = World::new(92, Faction::Assembly);
        world.ai_enabled = false;
        world.players[0].salvage = 5_000;
        world.players[0].pressure = 400;
        // A Drydock far from any water cannot launch a barge; one by the
        // border sea can.
        let inland = world.spawn_for_tests(0, Kind::Drydock, Pos::cell(25, 64));
        assert!(
            world
                .issue(
                    0,
                    Command::Train {
                        building: inland,
                        kind: Kind::Barge,
                    },
                )
                .is_err(),
            "no water within reach"
        );
        let barge = world.spawn_for_tests(0, Kind::Barge, Pos::cell(2, 64));
        let (bx, by) = world.entity(barge).unwrap().pos.cell_xy();
        assert_eq!(world.map.terrain(bx, by), Terrain::Deep);
        assert!(
            !world.cell_walkable_for(Kind::Barge, (10, 64)),
            "no hull on dry ground"
        );
        // Five machines board: four fit, the fifth is refused as full.
        let riders: Vec<u32> = (0..5)
            .map(|i| world.spawn_for_tests(0, Kind::Reedguard, Pos::cell(4 + i, 64)))
            .collect();
        issue_and_step(
            &mut world,
            0,
            Command::Board {
                units: riders.clone(),
                transport: barge,
            },
        );
        for _ in 0..200 {
            world.step();
        }
        let aboard: Vec<u32> = riders
            .iter()
            .copied()
            .filter(|id| world.entity(*id).is_some_and(|e| e.aboard == Some(barge)))
            .collect();
        assert_eq!(aboard.len(), 4, "a hold of four");
        assert_eq!(world.entity(barge).unwrap().cargo.len(), 4);
        let refused = riders
            .iter()
            .find(|id| !aboard.contains(id))
            .and_then(|id| world.entity(*id))
            .expect("the fifth rider stays on the field");
        assert_eq!(refused.order, Order::Idle, "the full hold clears its order");
        // Riders are off the field: unseen, untargetable, no orders.
        let rider = aboard[0];
        assert!(!world.entity_visible(1, rider));
        assert!(
            world
                .issue(
                    0,
                    Command::Move {
                        units: vec![rider],
                        target: Pos::cell(20, 64),
                        queued: false,
                    },
                )
                .is_err()
        );
        // The barge sails along the border sea and unloads on the shore.
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![barge],
                target: Pos::cell(2, 40),
                queued: false,
            },
        );
        for _ in 0..600 {
            world.step();
        }
        let at = world.entity(barge).unwrap().pos;
        assert!(at.cell_xy().1 < 60, "the barge moved along the water");
        for id in &aboard {
            assert_eq!(world.entity(*id).unwrap().pos, at, "riders ride");
        }
        issue_and_step(&mut world, 0, Command::Unload { transport: barge });
        let landed = aboard
            .iter()
            .filter(|id| world.entity(**id).is_some_and(|e| e.aboard.is_none()))
            .count();
        assert_eq!(landed, 4, "all four stand on the shore");
        assert!(world.entity(barge).unwrap().cargo.is_empty());
        // Back aboard, then the barge dies: the hold dies with it.
        issue_and_step(
            &mut world,
            0,
            Command::Board {
                units: aboard.clone(),
                transport: barge,
            },
        );
        for _ in 0..120 {
            world.step();
        }
        assert_eq!(world.entity(barge).unwrap().cargo.len(), 4);
        let crew_before = world.players[0].crew;
        if let Some(entity) = world.entity_mut(barge) {
            entity.hp = 0;
        }
        world.sink_cargo();
        world.entities.retain(|entity| entity.hp > 0);
        assert!(aboard.iter().all(|id| world.entity(*id).is_none()));
        assert_eq!(
            world.players[0].crew,
            crew_before.saturating_sub(4 * spec(Kind::Reedguard).crew),
            "the hold's crew is freed"
        );
    }

    #[test]
    fn a_lifter_crosses_deep_water_and_buildings() {
        let mut world = World::new(93, Faction::Union);
        world.ai_enabled = false;
        world.gate.owner = Some(1);
        world.gate.tide = Tide::Flood;
        let lifter = world.spawn_for_tests(0, Kind::Lifter, Pos::cell(45, 49));
        let riveter = world.spawn_for_tests(0, Kind::Riveter, Pos::cell(44, 49));
        assert!(
            world.cell_walkable_for(Kind::Lifter, (60, 49)),
            "flight over the deep lane"
        );
        assert!(
            world.cell_walkable_for(Kind::Lifter, (64, 30)),
            "flight over the lake"
        );
        issue_and_step(
            &mut world,
            0,
            Command::Board {
                units: vec![riveter],
                transport: lifter,
            },
        );
        for _ in 0..60 {
            world.step();
        }
        assert_eq!(world.entity(riveter).unwrap().aboard, Some(lifter));
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![lifter],
                target: Pos::cell(80, 49),
                queued: false,
            },
        );
        for _ in 0..900 {
            world.step();
            if world.entity(lifter).unwrap().pos.cell_xy().0 >= 79 {
                break;
            }
        }
        assert!(
            world.entity(lifter).unwrap().pos.cell_xy().0 >= 79,
            "the lifter crossed the flooded lane"
        );
        issue_and_step(&mut world, 0, Command::Unload { transport: lifter });
        let landed = world.entity(riveter).unwrap();
        assert!(landed.aboard.is_none());
        assert!(landed.pos.cell_xy().0 >= 76, "set down on the far bank");
    }

    #[test]
    fn the_hold_gauge_drains_when_a_mouth_is_lost_instead_of_resetting() {
        let mut world = World::new(41, Faction::Union);
        world.ai_enabled = false;
        world.gate.owner = Some(0);
        let (nx, ny) = CROSSING_MOUTHS[0][0].cell_xy();
        let (sx, sy) = CROSSING_MOUTHS[1][0].cell_xy();
        place(&mut world, 0, Kind::Riveter, nx, ny);
        let south = place(&mut world, 0, Kind::Riveter, sx, sy);
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(world.lane_hold[0], 100);
        assert_eq!(world.lane_hold[1], 0);
        world.entity_mut(south).unwrap().pos = Pos::cell(20, 64);
        for _ in 0..10 {
            world.step();
        }
        assert_eq!(
            world.lane_hold[0],
            100 - 10 * TIDE_HOLD_DRAIN_PER_TICK,
            "the gauge drains, it does not reset"
        );
        world.entity_mut(south).unwrap().pos = Pos::cell(sx, sy);
        for _ in 0..10 {
            world.step();
        }
        assert_eq!(world.lane_hold[0], 110 - 10 * TIDE_HOLD_DRAIN_PER_TICK);
        assert_eq!(world.lane_hold[1], 0);
    }

    #[test]
    fn crossing_mouths_are_lit_for_both_sides_without_report_spam() {
        let mut world = World::new(42, Faction::Union);
        world.ai_enabled = false;
        assert!(world.visible(0, CROSSING_MOUTHS[1][1]));
        assert!(world.visible(1, CROSSING_MOUTHS[0][0]));
        let (ex, ey) = CROSSING_MOUTHS[0][1].cell_xy();
        assert!(
            !world.visible(0, Pos::cell(ex + CROSSING_HOLD_RADIUS_CELLS + 1, ey)),
            "the light stops at the hold radius"
        );
        let enemy = place(&mut world, 1, Kind::Reedguard, ex, ey);
        assert!(world.entity_visible(0, enemy));
        world.step();
        let seen = world
            .events
            .iter()
            .filter(|e| e.kind == EventKind::EnemySeen && e.player == Some(0))
            .count();
        assert_eq!(seen, 1, "one report for the machine at the mouth");
        for _ in 0..30 {
            world.step();
            assert!(
                !world
                    .events
                    .iter()
                    .any(|e| e.kind == EventKind::EnemySeen && e.player == Some(0)),
                "a machine that stays in sight is not reported again"
            );
        }
    }

    #[test]
    fn a_caisson_holds_a_mouth_and_scouts_and_transports_do_not() {
        let mut world = World::new(43, Faction::Union);
        world.ai_enabled = false;
        world.gate.owner = Some(0);
        let (wx, wy) = CROSSING_MOUTHS[0][0].cell_xy();
        let (ex, ey) = CROSSING_MOUTHS[0][1].cell_xy();
        place(&mut world, 0, Kind::Caisson, wx, wy);
        assert!(world.holds_lane(0, 0), "a Caisson holds a mouth");
        place(&mut world, 1, Kind::Lampwright, ex, ey);
        assert!(world.holds_lane(0, 0), "an enemy scout does not contest");
        place(&mut world, 1, Kind::Barge, ex, ey);
        assert!(
            world.holds_lane(0, 0),
            "an enemy transport does not contest"
        );
        place(&mut world, 1, Kind::Caisson, ex, ey);
        assert!(
            !world.holds_lane(0, 0),
            "an enemy Caisson at the far mouth contests the lane"
        );
        assert!(!world.holds_lane(1, 0));
    }

    #[test]
    fn a_deposit_carries_its_load_and_an_emptied_wreck_is_announced_once() {
        let mut world = World::new(8, Faction::Union);
        world.ai_enabled = false;
        let worker = ids_for(&world, 0, Kind::Hook)[0];
        let Order::Gather { resource } = world.entity(worker).unwrap().order.clone() else {
            panic!("starting workers gather");
        };
        let wreck_pos = world
            .map
            .resources
            .iter()
            .find(|r| r.id == resource)
            .unwrap()
            .pos;
        for item in world.map.resources.iter_mut() {
            if item.id == resource {
                item.remaining = 3;
            }
        }
        for other in ids_for(&world, 0, Kind::Hook) {
            if other != worker {
                world.clear_order(other);
            }
        }
        let mut deposits = Vec::new();
        let mut emptied = Vec::new();
        for _ in 0..1500 {
            world.step();
            deposits.extend(
                world
                    .events
                    .iter()
                    .filter(|e| e.kind == EventKind::Deposit && e.entity == Some(worker))
                    .cloned(),
            );
            emptied.extend(
                world
                    .events
                    .iter()
                    .filter(|e| e.kind == EventKind::WreckEmptied)
                    .cloned(),
            );
            if !deposits.is_empty()
                && world
                    .entity(worker)
                    .is_some_and(|e| e.order != Order::Gather { resource })
            {
                break;
            }
        }
        assert_eq!(
            deposits.first().map(|e| e.amount),
            Some(3),
            "the deposit event carries its load"
        );
        assert_eq!(emptied.len(), 1, "the empty wreck is announced once");
        assert_eq!(emptied[0].other, Some(resource));
        assert_eq!(emptied[0].player, Some(0));
        assert_eq!(emptied[0].from, Some(wreck_pos));
        assert_eq!(emptied[0].text, "workers move on");
    }

    #[test]
    fn auto_gather_skips_lane_rim_and_island_wrecks_but_orders_and_dredgers_do_not() {
        let mut world = World::new(44, Faction::Union);
        world.ai_enabled = false;
        world.gate.tide = Tide::Open;
        world.gate.dry_arm = Arm::from_north(true);
        let lane_wreck = world
            .map
            .resources
            .iter()
            .find(|r| r.pos == Pos::cell(54, 49))
            .map(|r| r.id);
        assert!(lane_wreck.is_some());
        let beside = Pos::cell(53, 49);
        assert_eq!(world.nearest_salvage(0, Kind::Hook, beside, 3, true), None);
        assert_eq!(
            world.nearest_salvage(0, Kind::Hook, beside, 3, false),
            lane_wreck
        );
        assert_eq!(
            world.nearest_salvage(0, Kind::Dredger, beside, 3, true),
            lane_wreck
        );
        let deep_wreck = world
            .map
            .resources
            .iter()
            .find(|r| r.pos == Pos::cell(64, 56))
            .map(|r| r.id);
        assert!(deep_wreck.is_some());
        let island = Pos::cell(64, 58);
        assert_eq!(world.nearest_salvage(0, Kind::Hook, island, 3, true), None);
        assert_eq!(
            world.nearest_salvage(0, Kind::Hook, island, 3, false),
            deep_wreck
        );
        let on_its_own = world.spawn_order(0, Kind::Hook, beside, None);
        assert!(
            !matches!(on_its_own, Order::Gather { resource } if Some(resource) == lane_wreck),
            "a new worker never picks the lane wreck by itself, got {on_its_own:?}"
        );
        let rallied = world.spawn_order(0, Kind::Hook, Pos::cell(60, 60), Some(Pos::cell(55, 49)));
        assert!(
            matches!(rallied, Order::Gather { resource } if Some(resource) == lane_wreck),
            "a rally on the lane wreck is an order, got {rallied:?}"
        );
    }

    #[test]
    fn rules_15_mouth_scrap_overflow_flotsam_scout_and_hull_upgrades() {
        let mut world = World::new(46, Faction::Union);
        world.ai_enabled = false;
        // A machine that falls at a crossing mouth leaves half the scrap.
        let at_mouth = place(&mut world, 1, Kind::Reedguard, 78, 49);
        let fallen = world.entity(at_mouth).unwrap().clone();
        world.leave_scrap(&fallen, Some(0));
        assert_eq!(
            world.map.resources.last().unwrap().remaining,
            spec(Kind::Reedguard).salvage * SCRAP_BASE_PERCENT / 100 * MOUTH_SCRAP_PERCENT / 100
        );
        // Pressure income over a full bar becomes salvage at 3 per 10.
        for worker in ids_for(&world, 0, Kind::Hook) {
            world.clear_order(worker);
        }
        world.players[0].pressure = world.players[0].pressure_cap;
        let salvage = world.players[0].salvage;
        for _ in 0..60 * 30 {
            world.step();
        }
        // Less the headquarters' own 40 a minute (rules 20).
        let gained = world.players[0].salvage - salvage - HQ_SALVAGE_PER_MINUTE;
        assert!(
            (15..=21).contains(&gained),
            "a minute of 60 pressure over the cap is 18 salvage, got {gained}"
        );
        // A falling flood refills the lane wrecks, never above 900.
        let layout = world.map.layout();
        for resource in &mut world.map.resources {
            if layout.is_lane_wreck(resource.pos.cell_xy()) {
                resource.remaining = if resource.pos.cell_xy() == (54, 49) {
                    100
                } else {
                    LANE_WRECK - 50
                };
            }
        }
        world.gate.tide = Tide::Flood;
        world.gate.opened = true;
        world.gate.flood_until = Some(world.tick + 1);
        world.step();
        world.step();
        for resource in &world.map.resources {
            if resource.pos.cell_xy() == (54, 49) {
                assert_eq!(resource.remaining, 100 + FLOOD_FLOTSAM);
            } else if world.map.layout().is_lane_wreck(resource.pos.cell_xy()) {
                assert_eq!(resource.remaining, LANE_WRECK);
            }
        }
        // The headquarters trains the scout.
        let hq = ids_for(&world, 0, Kind::Headquarters)[0];
        world.players[0].salvage = 1_000;
        issue_and_step(
            &mut world,
            0,
            Command::Train {
                building: hq,
                kind: Kind::Tidewatch,
            },
        );
        assert_eq!(
            world
                .entity(hq)
                .unwrap()
                .queue
                .first()
                .map(|item| item.kind),
            Some(Kind::Tidewatch)
        );
        assert!(
            world
                .issue(
                    0,
                    Command::Train {
                        building: hq,
                        kind: Kind::Riveter,
                    },
                )
                .is_err()
        );
        // Plate reaches machines trained after it; REFIT adds 15% to every
        // combat machine; TEMPER adds 10% to every shot.
        world.players[0].upgrades = vec![Upgrade::Plate];
        let late = world.spawn_unit(0, Kind::Bulwark, Pos::cell(30, 60));
        assert_eq!(
            world.entity(late).unwrap().max_hp,
            spec(Kind::Bulwark).health * 125 / 100
        );
        let riveter = world.spawn_unit(0, Kind::Riveter, Pos::cell(31, 60));
        world.players[0].upgrades.push(Upgrade::Refit);
        world.apply_upgrade_effects(0, Upgrade::Refit);
        assert_eq!(
            world.entity(late).unwrap().max_hp,
            spec(Kind::Bulwark).health * 140 / 100
        );
        // Plate covers Union's Riveter too since rules 17.
        assert_eq!(
            world.entity(riveter).unwrap().max_hp,
            spec(Kind::Riveter).health * 140 / 100
        );
        world.players[0].upgrades.push(Upgrade::Temper);
        assert_eq!(world.tempered(0, Kind::Riveter, 20), 22);
        assert_eq!(world.tempered(0, Kind::Tower, 20), 20);
    }

    #[test]
    fn every_kill_leaves_scrap_and_the_upgrade_doubles_it() {
        let mut world = World::new(45, Faction::Union);
        world.ai_enabled = false;
        let enemy = place(&mut world, 1, Kind::Reedguard, 40, 60);
        let fallen = world.entity(enemy).unwrap().clone();
        let before = world.map.resources.len();
        world.leave_scrap(&fallen, Some(0));
        assert_eq!(world.map.resources.len(), before + 1);
        assert_eq!(
            world.map.resources.last().unwrap().remaining,
            spec(Kind::Reedguard).salvage * SCRAP_BASE_PERCENT / 100
        );
        world.players[0].upgrades.push(Upgrade::ScrapRecovery);
        world.leave_scrap(&fallen, Some(0));
        assert_eq!(
            world.map.resources.last().unwrap().remaining,
            spec(Kind::Reedguard).salvage * SCRAP_UPGRADED_PERCENT / 100
        );
        let count = world.map.resources.len();
        world.leave_scrap(&fallen, None);
        let hook = world
            .entity(ids_for(&world, 0, Kind::Hook)[0])
            .unwrap()
            .clone();
        world.leave_scrap(&hook, Some(1));
        assert_eq!(
            world.map.resources.len(),
            count,
            "no scrap without a killer or from a worker"
        );
    }

    #[test]
    fn reclaim_turns_pressure_into_salvage_at_once_and_refuses_when_short() {
        let mut world = World::new(46, Faction::Union);
        world.ai_enabled = false;
        for worker in ids_for(&world, 0, Kind::Hook) {
            world.clear_order(worker);
        }
        let hq = ids_for(&world, 0, Kind::Headquarters)[0];
        world.players[0].pressure = 150;
        let salvage = world.players[0].salvage;
        world
            .issue(0, Command::Reclaim { building: hq })
            .expect("RECLAIM is legal with the pressure in hand");
        let mut reclaimed = None;
        for _ in 0..5 {
            world.step();
            if let Some(event) = world.events.iter().find(|e| e.kind == EventKind::Reclaimed) {
                reclaimed = Some(event.clone());
                break;
            }
        }
        let reclaimed = reclaimed.expect("the reclaim applied");
        assert_eq!(reclaimed.amount, RECLAIM_SALVAGE as i32);
        assert_eq!(reclaimed.entity, Some(hq));
        assert_eq!(reclaimed.player, Some(0));
        assert!((50..=51).contains(&world.players[0].pressure));
        assert_eq!(world.players[0].salvage, salvage + RECLAIM_SALVAGE);
        let short = world
            .issue(0, Command::Reclaim { building: hq })
            .unwrap_err();
        assert!(short.contains("insufficient pressure"), "{short}");
        world.players[0].pressure = 150;
        let enemy_hq = ids_for(&world, 1, Kind::Headquarters)[0];
        assert!(
            world
                .issue(0, Command::Reclaim { building: enemy_hq })
                .is_err(),
            "only an own headquarters reclaims"
        );
        world
            .issue(0, Command::Reclaim { building: hq })
            .expect("no cooldown on RECLAIM");
    }

    #[test]
    fn attack_move_puts_specialists_in_the_rear_rank() {
        let mut world = World::new(47, Faction::Union);
        world.ai_enabled = false;
        let riveters: Vec<u32> = (58..=61)
            .map(|y| place(&mut world, 0, Kind::Riveter, 30, y))
            .collect();
        let bulwark = place(&mut world, 0, Kind::Bulwark, 30, 62);
        let mut units = riveters.clone();
        units.push(bulwark);
        fn target_x(world: &World, id: u32) -> i32 {
            match world.entity(id).unwrap().order {
                Order::AttackMove { target } | Order::Move { target } => target.cell_xy().0,
                ref other => panic!("expected a move order, got {other:?}"),
            }
        }
        issue_and_step(
            &mut world,
            0,
            Command::AttackMove {
                units: units.clone(),
                target: Pos::cell(60, 60),
                queued: false,
            },
        );
        for _ in 0..3 {
            if matches!(
                world.entity(bulwark).unwrap().order,
                Order::AttackMove { .. }
            ) {
                break;
            }
            world.step();
        }
        let behind = target_x(&world, bulwark);
        let front = riveters
            .iter()
            .map(|id| target_x(&world, *id))
            .min()
            .unwrap();
        assert!(
            behind < front,
            "eastward, the Bulwark's slot ({behind}) lies behind the Riveters ({front})"
        );
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units,
                target: Pos::cell(5, 60),
                queued: false,
            },
        );
        for _ in 0..3 {
            if matches!(world.entity(bulwark).unwrap().order, Order::Move { .. }) {
                break;
            }
            world.step();
        }
        let behind = target_x(&world, bulwark);
        let front = riveters
            .iter()
            .map(|id| target_x(&world, *id))
            .max()
            .unwrap();
        assert!(
            behind > front,
            "westward, the Bulwark's slot ({behind}) lies behind the Riveters ({front})"
        );
    }

    #[test]
    fn death_events_name_the_killer_and_its_kind() {
        let mut world = World::new(48, Faction::Union);
        world.ai_enabled = false;
        let riveter = place(&mut world, 0, Kind::Riveter, 30, 60);
        let victim = place(&mut world, 1, Kind::Reedguard, 31, 60);
        world.entity_mut(victim).unwrap().hp = 1;
        let mut death = None;
        for _ in 0..120 {
            world.step();
            if let Some(event) = world
                .events
                .iter()
                .find(|e| e.kind == EventKind::Death && e.entity == Some(victim))
            {
                death = Some(event.clone());
                break;
            }
        }
        let death = death.expect("the Reedguard dies to the Riveter");
        assert_eq!(death.other, Some(riveter));
        assert_eq!(death.cause, Some(Kind::Riveter));

        let loom = place(&mut world, 1, Kind::Loom, 100, 40);
        let shelled = place(&mut world, 0, Kind::Sounder, 20, 70);
        world.entity_mut(shelled).unwrap().hp = 1;
        world.artillery.push(ArtilleryShot {
            owner: 1,
            source: loom,
            from: Pos::cell(100, 40),
            target: Pos::cell(20, 70),
            impact_tick: world.tick,
        });
        let mut death = None;
        for _ in 0..5 {
            world.step();
            if let Some(event) = world
                .events
                .iter()
                .find(|e| e.kind == EventKind::Death && e.entity == Some(shelled))
            {
                death = Some(event.clone());
                break;
            }
        }
        let death = death.expect("the shell lands");
        assert_eq!(death.other, Some(loom));
        assert_eq!(death.cause, Some(Kind::Loom));

        let barge = place(&mut world, 1, Kind::Barge, 100, 44);
        let rider = place(&mut world, 1, Kind::Reedguard, 100, 44);
        world.entity_mut(rider).unwrap().aboard = Some(barge);
        world.entity_mut(barge).unwrap().cargo.push(rider);
        world.entity_mut(barge).unwrap().hp = 0;
        world.sink_cargo();
        let sunk = world
            .events
            .iter()
            .find(|e| e.kind == EventKind::Death && e.entity == Some(rider))
            .expect("the hold sinks with the barge");
        assert_eq!(sunk.other, Some(barge));
        assert_eq!(sunk.cause, Some(Kind::Barge));
    }

    #[test]
    fn the_tide_hold_needs_the_station_as_well_as_the_mouths() {
        let mut world = World::new(51, Faction::Union);
        world.ai_enabled = false;
        for mouths in CROSSING_MOUTHS {
            let (x, y) = mouths[0].cell_xy();
            place(&mut world, 0, Kind::Riveter, x, y);
        }
        assert!(
            !world.holds_lane(0, 0),
            "machines at both mouths hold nothing without the station"
        );
        world.step();
        assert_eq!(world.lane_hold[0], 0);
        world.gate.owner = Some(0);
        assert!(world.holds_lane(0, 0) && world.holds_lane(0, 1));
        world.step();
        assert_eq!(world.lane_hold[0], 1, "the station starts the count");
        // Losing the station stops it, even with the mouths still held.
        world.gate.owner = Some(1);
        assert!(!world.holds_lane(0, 0));
        assert!(
            !world.holds_lane(1, 0),
            "the new holder still needs a machine at a mouth"
        );
        world.step();
        assert_eq!(world.lane_hold[0], 0);
    }

    #[test]
    fn artillery_on_an_attack_move_stops_at_its_reach_and_deploys() {
        let mut world = World::new(52, Faction::Assembly);
        world.ai_enabled = false;
        let loom = place(&mut world, 0, Kind::Loom, 40, 60);
        let target = place(&mut world, 1, Kind::Reedguard, 46, 60);
        let range = world.weapon_range(&world.entity(loom).unwrap().clone());
        assert!(
            world
                .entity(loom)
                .unwrap()
                .pos
                .distance_sq(world.entity(target).unwrap().pos)
                <= i64::from(range).pow(2),
            "the target starts inside the Loom's reach"
        );
        issue_and_step(
            &mut world,
            0,
            Command::AttackMove {
                units: vec![loom],
                target: Pos::cell(60, 60),
                queued: false,
            },
        );
        let mut deployed_at = None;
        for _ in 0..120 {
            world.step();
            let entity = world.entity(loom).unwrap();
            if entity.deployed {
                deployed_at = Some(entity.pos);
                break;
            }
        }
        let deployed_at = deployed_at.expect("the Loom deploys where it can fire");
        assert!(
            deployed_at.distance_sq(world.entity(target).unwrap().pos) <= i64::from(range).pow(2),
            "it stopped inside its own reach instead of walking in"
        );
        assert!(
            matches!(world.entity(loom).unwrap().order, Order::Deploy),
            "the attack-move became a firing stance"
        );
        // A plain move is the player's word and is not overridden.
        let walker = place(&mut world, 0, Kind::Loom, 40, 64);
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![walker],
                target: Pos::cell(60, 64),
                queued: false,
            },
        );
        for _ in 0..60 {
            world.step();
        }
        assert!(
            !world.entity(walker).unwrap().deployed,
            "a plain move keeps walking"
        );
    }

    #[test]
    fn an_attack_move_holds_the_group_to_its_slowest_machine() {
        let mut world = World::new(53, Faction::Union);
        world.ai_enabled = false;
        let riveter = place(&mut world, 0, Kind::Riveter, 20, 60);
        let caisson = place(&mut world, 0, Kind::Caisson, 20, 62);
        assert!(spec(Kind::Caisson).speed < spec(Kind::Riveter).speed);
        issue_and_step(
            &mut world,
            0,
            Command::AttackMove {
                units: vec![riveter, caisson],
                target: Pos::cell(34, 61),
                queued: false,
            },
        );
        assert_eq!(
            world.entity(riveter).unwrap().pace,
            spec(Kind::Caisson).speed,
            "the group takes the slowest machine's pace"
        );
        let start = world.entity(riveter).unwrap().pos;
        for _ in 0..120 {
            world.step();
        }
        let fast = world.entity(riveter).unwrap().pos.distance_sq(start);
        let slow = world
            .entity(caisson)
            .unwrap()
            .pos
            .distance_sq(Pos::cell(20, 62));
        assert!(
            fast <= slow.saturating_mul(2),
            "the Riveter no longer runs ahead: {fast} against {slow}"
        );
        // A plain move releases the pace.
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![riveter],
                target: Pos::cell(20, 60),
                queued: false,
            },
        );
        assert_eq!(world.entity(riveter).unwrap().pace, 0);
    }

    #[test]
    fn a_deployed_line_does_not_pen_its_own_machines_and_a_blocked_move_says_so() {
        let mut world = World::new(54, Faction::Assembly);
        world.ai_enabled = false;
        // A Reedguard ringed by its own deployed Looms must still get out.
        let penned = place(&mut world, 0, Kind::Reedguard, 40, 60);
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let loom = place(&mut world, 0, Kind::Loom, 40 + dx, 60 + dy);
            if let Some(entity) = world.entity_mut(loom) {
                entity.deployed = true;
            }
        }
        issue_and_step(
            &mut world,
            0,
            Command::Move {
                units: vec![penned],
                target: Pos::cell(48, 60),
                queued: false,
            },
        );
        let start = world.entity(penned).unwrap().pos;
        for _ in 0..600 {
            world.step();
            if world.entity(penned).unwrap().pos != start {
                break;
            }
        }
        assert_ne!(
            world.entity(penned).unwrap().pos,
            start,
            "an own deployed line is not a wall to its own machines"
        );
        // Ringed by buildings instead, the machine says it cannot go.
        let mut world = World::new(55, Faction::Assembly);
        world.ai_enabled = false;
        let stuck = place(&mut world, 0, Kind::Reedguard, 40, 60);
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            place(&mut world, 0, Kind::Palisade, 40 + dx, 60 + dy);
        }
        world
            .issue(
                0,
                Command::Move {
                    units: vec![stuck],
                    target: Pos::cell(48, 60),
                    queued: false,
                },
            )
            .expect("the move is legal to issue");
        let mut said = false;
        for _ in 0..900 {
            world.step();
            if world
                .events
                .iter()
                .any(|e| e.kind == EventKind::PathBlocked && e.entity == Some(stuck))
            {
                said = true;
                break;
            }
        }
        assert!(said, "a machine that cannot reach its target says so");
        assert!(matches!(world.entity(stuck).unwrap().order, Order::Idle));
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;
    use bw_content::{
        DOCTRINE_PRESSURE, DOCTRINE_SALVAGE, DOCTRINE_TICKS, Doctrine, LOOM_BLAST_RADIUS,
        LOOM_WINDUP_TICKS, SURGE_COOLDOWN_TICKS, SURGE_PRESSURE, SURGE_TICKS,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unit(world: &mut World, owner: u8, kind: Kind, cell: (i32, i32)) -> u32 {
        let id = world.spawn_unit(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(entity) = world.entity_mut(id) {
            entity.order = Order::Idle;
        }
        id
    }

    fn fixture(world: &mut World) {
        world.ai_enabled = false;
        world.reset_fixture_origin().expect("clean fixture origin");
    }

    fn hq(world: &World, owner: u8) -> u32 {
        world
            .entities
            .iter()
            .find(|entity| entity.owner == owner && entity.kind == Kind::Headquarters)
            .map(|entity| entity.id)
            .expect("starting headquarters")
    }

    fn advance_to_impact(world: &mut World) -> u64 {
        let launch_tick = world
            .events
            .iter()
            .find(|event| event.kind == EventKind::ArtilleryWarning)
            .map(|event| event.tick)
            .expect("artillery warning");
        while !world.artillery.is_empty() {
            world.step();
        }
        world
            .events
            .iter()
            .find(|event| event.kind == EventKind::Shot)
            .map(|event| event.tick - launch_tick)
            .expect("impact shot")
    }

    #[test]
    fn bulwark_arc_is_committed_for_fire_and_damage() {
        let mut world = World::new(700, Faction::Union);
        let bulwark = unit(&mut world, 0, Kind::Bulwark, (40, 64));
        let enemy = unit(&mut world, 1, Kind::Reedguard, (40, 66));
        if let Some(entity) = world.entity_mut(bulwark) {
            entity.deployed = true;
            entity.facing = 0;
        }
        let max_hp = world.entity(enemy).expect("enemy").max_hp;
        fixture(&mut world);
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![bulwark],
                    target: enemy,
                },
            )
            .expect("behind target remains selectable");
        world.step();
        assert_eq!(world.entity(enemy).expect("enemy survives").hp, max_hp);
        assert_eq!(world.entity(bulwark).expect("bulwark").facing, 0);

        if let Some(target) = world.entity_mut(enemy) {
            target.pos = Pos::cell(40, 62);
        }
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![bulwark],
                    target: enemy,
                },
            )
            .expect("front target selectable");
        world.step();
        assert_eq!(world.entity(enemy).expect("enemy").hp, max_hp - 10);
        assert_eq!(
            world.entity(bulwark).expect("bulwark").facing,
            0,
            "deployed Bulwark fire cannot auto-rotate"
        );

        let mut incoming = World::new(701, Faction::Union);
        let shield = unit(&mut incoming, 0, Kind::Bulwark, (40, 64));
        let attacker = unit(&mut incoming, 1, Kind::Riveter, (40, 62));
        if let Some(entity) = incoming.entity_mut(shield) {
            entity.deployed = true;
            entity.facing = 0;
        }
        if let Some(entity) = incoming.entity_mut(attacker) {
            entity.order = Order::Attack { target: shield };
        }
        fixture(&mut incoming);
        let hp = incoming.entity(shield).expect("shield").hp;
        incoming.step();
        assert_eq!(
            incoming.entity(shield).expect("shield").hp,
            hp - spec(Kind::Riveter).damage / 2,
            "front arc halves incoming damage"
        );

        let oblique = incoming.entity(shield).expect("shield").clone();
        if let Some(entity) = incoming.entity_mut(shield) {
            entity.facing = 1;
        }
        if let Some(entity) = incoming.entity_mut(attacker) {
            entity.pos = Pos::raw(
                oblique.pos.x.saturating_sub(3 * FP),
                oblique.pos.y.saturating_sub(FP),
            );
            entity.attack_cooldown = 0;
            entity.order = Order::Attack { target: shield };
        }
        incoming.step();
        assert_eq!(
            incoming.entity(shield).expect("shield").hp,
            hp - spec(Kind::Riveter).damage / 2 - spec(Kind::Riveter).damage,
            "a rear oblique attacker is outside a diagonal edge, not inside by octant"
        );
    }

    #[test]
    fn loom_requires_deployment_and_commits_fixed_ground_blasts() {
        let mut world = World::new(702, Faction::Union);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let victim = unit(&mut world, 1, Kind::Reedguard, (45, 64));
        fixture(&mut world);
        let hp = world.entity(victim).expect("victim").hp;
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: victim,
                },
            )
            .expect("packed Loom attack command");
        world.step();
        assert!(world.artillery.is_empty(), "packed Loom cannot launch");
        assert_eq!(world.entity(victim).expect("victim").hp, hp);
        // Rules 19: a packed Loom sent at a target in its reach deploys.
        assert!(world.entity(loom).expect("loom").deploy_target);

        if let Some(entity) = world.entity_mut(victim) {
            entity.pos = Pos::cell(41, 64);
        }
        if let Some(entity) = world.entity_mut(loom) {
            entity.deployed = true;
            entity.deploy_remaining = 0;
        }
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: victim,
                },
            )
            .expect("minimum-range command remains legal");
        world.step();
        assert!(
            world.artillery.is_empty(),
            "Loom dead zone rejects a one-cell target"
        );

        if let Some(entity) = world.entity_mut(victim) {
            entity.pos = Pos::cell(45, 64);
        }
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: victim,
                },
            )
            .expect("deployed Loom target");
        world.step();
        assert_eq!(world.artillery.len(), 1);
        let shot = world.artillery[0].clone();
        assert_eq!(
            shot.impact_tick - world.events[0].tick,
            u64::from(LOOM_WINDUP_TICKS)
        );
        assert!(world.events.iter().any(|event| {
            event.kind == EventKind::ArtilleryWarning && event.entity == Some(loom)
        }));
        assert_eq!(world.entity(victim).expect("victim").hp, hp);
    }

    #[test]
    fn loom_blast_is_clustered_fixed_and_survives_caster_death() {
        let mut world = World::new(703, Faction::Union);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let first = unit(&mut world, 1, Kind::Reedguard, (45, 64));
        let second = unit(&mut world, 1, Kind::Reedguard, (45, 65));
        let friendly = unit(&mut world, 0, Kind::Riveter, (46, 64));
        if let Some(entity) = world.entity_mut(loom) {
            entity.deployed = true;
        }
        fixture(&mut world);
        // Idle machines engage on their own, so the bystanders are put under
        // Surge (weapons off, no pursuit) for longer than the windup: only the
        // blast may change their hull.
        for id in [first, second, friendly] {
            if let Some(entity) = world.entity_mut(id) {
                entity.surge_remaining = SURGE_TICKS;
            }
        }
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: first,
                },
            )
            .expect("Loom attack");
        world.step();
        let first_hp = world.entity(first).expect("first").hp;
        let second_hp = world.entity(second).expect("second").hp;
        let friendly_hp = world.entity(friendly).expect("friendly").hp;
        if let Some(entity) = world.entity_mut(loom) {
            entity.hp = 0;
        }
        let elapsed = advance_to_impact(&mut world);
        assert_eq!(elapsed, u64::from(LOOM_WINDUP_TICKS));
        assert_eq!(
            world.entity(first).expect("first").hp,
            first_hp - spec(Kind::Loom).damage
        );
        assert_eq!(
            world.entity(second).expect("second").hp,
            second_hp - spec(Kind::Loom).damage
        );
        assert_eq!(world.entity(friendly).expect("friendly").hp, friendly_hp);
        assert!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Shot)
                .count()
                == 1,
            "one shell emits one impact shot event"
        );
        assert!(
            world
                .events
                .iter()
                .any(|event| { event.kind == EventKind::Damage && event.entity == Some(first) })
        );
        assert_eq!(LOOM_BLAST_RADIUS, 320);
    }

    #[test]
    fn loom_ground_point_does_not_follow_a_target_into_fog() {
        let mut world = World::new(704, Faction::Union);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let victim = unit(&mut world, 1, Kind::Reedguard, (45, 64));
        if let Some(entity) = world.entity_mut(loom) {
            entity.deployed = true;
        }
        fixture(&mut world);
        let hp = world.entity(victim).expect("victim").hp;
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: victim,
                },
            )
            .expect("visible target");
        world.step();
        let announced = world.artillery[0].target;
        if let Some(entity) = world.entity_mut(victim) {
            entity.pos = Pos::cell(100, 20);
        }
        assert!(!world.visible(0, Pos::cell(100, 20)));
        world.step();
        assert!(
            world
                .knowledge(0)
                .visible
                .iter()
                .all(|entity| entity.id != victim)
        );
        while !world.artillery.is_empty() {
            world.step();
        }
        assert_eq!(world.entity(victim).expect("hidden victim").hp, hp);
        assert_eq!(announced, Pos::cell(45, 64));
    }

    #[test]
    fn formation_slots_are_unique_deterministic_and_face_stops_without_walking() {
        let mut world = World::new(705, Faction::Union);
        let mut ids = Vec::new();
        for i in 0..12 {
            ids.push(unit(&mut world, 0, Kind::Riveter, (20 + i, 20)));
        }
        fixture(&mut world);
        world
            .issue(
                0,
                Command::SetFormation {
                    units: ids.clone(),
                    formation: Formation::Loose,
                },
            )
            .expect("formation choice");
        world.step();
        assert!(
            ids.iter()
                .all(|id| { world.entity(*id).expect("unit").formation == Formation::Loose })
        );
        world
            .issue(
                0,
                Command::Move {
                    units: ids.clone(),
                    target: Pos::cell(100, 30),
                    queued: false,
                },
            )
            .expect("loose move");
        world.step();
        let slots: BTreeSet<(i32, i32)> = ids
            .iter()
            .map(|id| match world.entity(*id).expect("unit").order {
                Order::Move { target } => target.cell_xy(),
                _ => panic!("move order retained"),
            })
            .collect();
        assert_eq!(
            slots.len(),
            ids.len(),
            "large selections cannot overlap slots"
        );

        world
            .issue(
                0,
                Command::Move {
                    units: ids.clone(),
                    target: Pos::cell(90, 30),
                    queued: false,
                },
            )
            .expect("active move");
        world.step();
        world
            .issue(
                0,
                Command::Move {
                    units: ids.clone(),
                    target: Pos::cell(80, 30),
                    queued: true,
                },
            )
            .expect("queued move");
        world.step();
        assert!(ids.iter().all(|id| {
            world
                .entity(*id)
                .is_some_and(|entity| entity.waypoints.len() == 1)
        }));
        let face_target = Pos::cell(20, 30);
        world
            .issue(
                0,
                Command::Face {
                    units: ids.clone(),
                    target: face_target,
                },
            )
            .expect("face order");
        let before = world.entity(ids[0]).expect("unit").pos;
        world.step();
        let faced = world.entity(ids[0]).expect("unit");
        assert_eq!(faced.order, Order::Idle);
        assert_eq!(faced.pos, before, "Face never walks to its direction point");
        assert!(faced.waypoints.is_empty(), "Face clears queued waypoints");
        assert_eq!(
            faced.facing, 4,
            "direction is chosen from fixed-point target"
        );

        let specialist = unit(&mut world, 0, Kind::Bulwark, (40, 64));
        if let Some(entity) = world.entity_mut(specialist) {
            entity.deployed = true;
            entity.facing = 0;
        }
        assert!(
            world
                .issue(
                    0,
                    Command::Face {
                        units: vec![specialist],
                        target: Pos::cell(40, 40),
                    },
                )
                .is_err(),
            "deployed specialists must pack before facing"
        );
    }

    #[test]
    fn surge_is_atomic_exactly_timed_and_blocks_fire_and_deploy() {
        let mut world = World::new(706, Faction::Union);
        let first = unit(&mut world, 0, Kind::Riveter, (40, 64));
        let second = unit(&mut world, 0, Kind::Sounder, (40, 66));
        let victim = unit(&mut world, 1, Kind::Reedguard, (45, 64));
        fixture(&mut world);
        let pressure = world.players[0].pressure;
        let hp = world.entity(victim).expect("victim").hp;
        world
            .issue(
                0,
                Command::Surge {
                    units: vec![first, second],
                },
            )
            .expect("two packed machines can surge");
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![first],
                    target: victim,
                },
            )
            .expect("attack queued with Surge");
        world.step();
        assert_eq!(
            world.players[0].pressure,
            pressure - 2 * SURGE_PRESSURE,
            "cost is paid once for the whole atomic selection"
        );
        assert_eq!(
            world.entity(first).expect("first").surge_remaining,
            SURGE_TICKS - 1
        );
        assert_eq!(
            world.entity(first).expect("first").surge_cooldown,
            SURGE_COOLDOWN_TICKS - 1
        );
        assert_eq!(world.entity(victim).expect("victim").hp, hp);
        // A surging machine takes a capture order since rules 14: it runs
        // on and channels once the surge ends.
        assert!(
            world
                .issue(0, Command::Capture { units: vec![first] })
                .is_ok()
        );
        assert!(
            world
                .issue(0, Command::Deploy { units: vec![first] })
                .is_err()
        );

        let before_pressure = world.players[0].pressure;
        if let Some(entity) = world.entity_mut(second) {
            entity.surge_cooldown = 2;
        }
        assert!(
            world
                .issue(
                    0,
                    Command::Surge {
                        units: vec![first, second],
                    },
                )
                .is_err(),
            "one ineligible unit rejects the whole selection"
        );
        assert_eq!(world.players[0].pressure, before_pressure);

        for _ in 0..(SURGE_TICKS as usize - 1) {
            world.step();
        }
        assert_eq!(world.entity(first).expect("first").surge_remaining, 0);
        for _ in 0..(SURGE_COOLDOWN_TICKS - SURGE_TICKS) {
            world.step();
        }
        assert_eq!(world.entity(first).expect("first").surge_cooldown, 0);
        world
            .issue(0, Command::Surge { units: vec![first] })
            .expect("cooldown expires on the advertised tick");
    }

    #[test]
    fn doctrine_research_pauses_workers_refunds_and_applies_benefits() {
        let mut world = World::new(707, Faction::Union);
        world.ai_enabled = false;
        world.players[0].salvage = 300;
        world.players[0].pressure = 100;
        let headquarters = hq(&world, 0);
        world.reset_fixture_origin().expect("clean doctrine origin");
        world
            .issue(
                0,
                Command::Train {
                    building: headquarters,
                    kind: Kind::Hook,
                },
            )
            .expect("worker queued");
        world.step();
        let queued_remaining = world
            .entity(headquarters)
            .expect("hq")
            .queue
            .first()
            .expect("queued worker")
            .remaining;
        let salvage_before_research = world.players[0].salvage;
        let pressure_before_research = world.players[0].pressure;
        world
            .issue(
                0,
                Command::Research {
                    building: headquarters,
                    doctrine: Doctrine::Hauling,
                },
            )
            .expect("doctrine research");
        world.step();
        assert_eq!(
            world.players[0].salvage,
            salvage_before_research - DOCTRINE_SALVAGE
        );
        assert_eq!(
            world.players[0].pressure,
            pressure_before_research - DOCTRINE_PRESSURE
        );
        assert_eq!(
            world
                .entity(headquarters)
                .expect("hq")
                .queue
                .first()
                .expect("queue retained")
                .remaining,
            queued_remaining,
            "HQ worker production pauses while research is active"
        );
        assert_eq!(
            world.players[0].research.expect("research").remaining,
            DOCTRINE_TICKS - 1
        );
        for _ in 0..(DOCTRINE_TICKS - 1) {
            world.step();
        }
        assert_eq!(world.players[0].doctrine, Some(Doctrine::Hauling));
        assert!(world.players[0].research.is_none());
        assert!(world.events.iter().any(|event| {
            event.kind == EventKind::ResearchCompleted && event.entity == Some(headquarters)
        }));
        world.step();
        assert!(
            world
                .entity(headquarters)
                .expect("hq")
                .queue
                .first()
                .expect("queue remains until spawn")
                .remaining
                < queued_remaining
        );
        assert_eq!(world.carry_for(0, Kind::Hook), 8);
        assert!(
            world
                .issue(
                    0,
                    Command::Research {
                        building: headquarters,
                        doctrine: Doctrine::FireControl,
                    }
                )
                .is_err()
        );

        let mut cancelled = World::new(708, Faction::Union);
        cancelled.ai_enabled = false;
        cancelled.players[0].salvage = DOCTRINE_SALVAGE + 1;
        cancelled.players[0].pressure = DOCTRINE_PRESSURE;
        let cancel_hq = hq(&cancelled, 0);
        cancelled.reset_fixture_origin().expect("cancel origin");
        cancelled
            .issue(
                0,
                Command::Research {
                    building: cancel_hq,
                    doctrine: Doctrine::FireControl,
                },
            )
            .expect("research starts");
        cancelled.step();
        let salvage_after_start = cancelled.players[0].salvage;
        let pressure_after_start = cancelled.players[0].pressure;
        cancelled
            .issue(
                0,
                Command::CancelResearch {
                    building: cancel_hq,
                },
            )
            .expect("explicit cancellation");
        cancelled.step();
        assert_eq!(
            cancelled.players[0].salvage,
            salvage_after_start + DOCTRINE_SALVAGE * 3 / 4
        );
        assert_eq!(
            cancelled.players[0].pressure,
            pressure_after_start + DOCTRINE_PRESSURE * 3 / 4
        );
        assert!(cancelled.players[0].research.is_none());
        assert!(
            cancelled
                .events
                .iter()
                .any(|event| { event.kind == EventKind::ResearchCancelled })
        );
    }

    #[test]
    fn depth_commands_save_resume_and_replay_with_active_state() {
        let mut world = World::new(709, Faction::Union);
        world.ai_enabled = false;
        world.players[0].salvage = 300;
        world.players[0].pressure = 100;
        let headquarters = hq(&world, 0);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let surge_unit = unit(&mut world, 0, Kind::Riveter, (40, 66));
        let victim = unit(&mut world, 1, Kind::Reedguard, (45, 64));
        if let Some(entity) = world.entity_mut(loom) {
            entity.deployed = true;
        }
        world.reset_fixture_origin().expect("depth replay origin");
        world
            .issue(
                0,
                Command::Research {
                    building: headquarters,
                    doctrine: Doctrine::FireControl,
                },
            )
            .expect("research command");
        world.step();
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![loom],
                    target: victim,
                },
            )
            .expect("artillery command");
        world.step();
        assert!(!world.artillery.is_empty());
        world
            .issue(
                0,
                Command::Surge {
                    units: vec![surge_unit],
                },
            )
            .expect("surge command");
        world.step();
        assert!(!world.artillery.is_empty());
        assert!(world.players[0].research.is_some());
        assert!(
            world
                .entity(surge_unit)
                .expect("surge unit")
                .surge_remaining
                > 0
        );
        let root = std::env::temp_dir();
        let stamp = format!("brinewake-depth-{}-{}", std::process::id(), world.tick);
        let save = root.join(format!("{stamp}.save"));
        let replay = root.join(format!("{stamp}.replay"));
        world.save(&save).expect("active depth save");
        let mut loaded = World::load(&save).expect("active depth load");
        assert_eq!(world.state_hash(), loaded.state_hash());
        for _ in 0..40 {
            world.step();
            loaded.step();
            assert_eq!(world.state_hash(), loaded.state_hash());
        }
        world.export_replay(&replay).expect("depth replay export");
        let replayed = World::replay(&replay).expect("depth replay load");
        assert_eq!(world.state_hash(), replayed.state_hash());
        let _ = fs::remove_file(save);
        let _ = fs::remove_file(replay);
    }

    #[test]
    fn malformed_depth_state_is_rejected_before_save() {
        let mut facing = World::new(710, Faction::Union);
        facing.entity_mut(1).expect("hq").facing = 8;
        assert!(facing.validate_invariants().is_err());

        let mut research = World::new(711, Faction::Union);
        let headquarters = hq(&research, 0);
        research.players[0].research = Some(Research {
            building: headquarters,
            doctrine: Doctrine::Hauling,
            remaining: 0,
            tier: 1,
        });
        assert!(research.validate_invariants().is_err());

        let mut artillery = World::new(712, Faction::Union);
        let source = artillery.next_entity_id;
        artillery.artillery.push(ArtilleryShot {
            owner: 0,
            source,
            from: Pos::cell(40, 64),
            target: Pos::cell(45, 64),
            impact_tick: LOOM_WINDUP_TICKS as u64 + 1,
        });
        assert!(artillery.validate_invariants().is_err());
    }

    #[test]
    fn same_tick_surge_then_deploy_is_rejected_at_execution_and_capture_chains() {
        for deploy in [true, false] {
            let mut world = World::new(if deploy { 713 } else { 714 }, Faction::Union);
            let unit_kind = if deploy { Kind::Bulwark } else { Kind::Riveter };
            let unit_id = unit(&mut world, 0, unit_kind, (40, 64));
            fixture(&mut world);
            world
                .issue(
                    0,
                    Command::Surge {
                        units: vec![unit_id],
                    },
                )
                .expect("Surge submission");
            if deploy {
                world
                    .issue(
                        0,
                        Command::Deploy {
                            units: vec![unit_id],
                        },
                    )
                    .expect("Deploy submission before Surge executes");
            } else {
                world
                    .issue(
                        0,
                        Command::Capture {
                            units: vec![unit_id],
                        },
                    )
                    .expect("Capture submission before Surge executes");
            }
            world.step();
            let surge = world
                .command_log
                .iter()
                .find(|record| matches!(record.command, Command::Surge { .. }))
                .expect("Surge record");
            assert!(surge.accepted);
            assert_eq!(surge.applied, Some(true));
            let follow_up = world
                .command_log
                .iter()
                .find(|record| {
                    if deploy {
                        matches!(record.command, Command::Deploy { .. })
                    } else {
                        matches!(record.command, Command::Capture { .. })
                    }
                })
                .expect("same-tick follow-up record");
            assert!(follow_up.accepted);
            // Deploy is revalidated against the active Surge; a capture
            // chains after it (rules 14).
            assert_eq!(
                follow_up.applied,
                Some(!deploy),
                "execution must revalidate active Surge"
            );
        }
    }

    #[test]
    fn surge_preserves_fixed_point_terrain_penalties_and_flood_ratio() {
        fn aggregate_speed(kind: Kind, cell: (i32, i32), surge: bool) -> u64 {
            let mut world = World::new(715, Faction::Union);
            world.ai_enabled = false;
            world.gate.tide = Tide::Open;
            world.gate.dry_arm = Arm::from_north(true);
            let id = unit(&mut world, 0, kind, cell);
            let mut snapshot = world.entity(id).expect("speed fixture unit").clone();
            snapshot.surge_remaining = if surge { SURGE_TICKS } else { 0 };
            (0..300u64)
                .map(|tick| {
                    world.tick = tick;
                    world.speed_per_tick(&snapshot) as u64
                })
                .sum()
        }

        for kind in [Kind::Riveter, Kind::Sounder, Kind::Skipper] {
            let salt = (
                aggregate_speed(kind, (40, 40), false),
                aggregate_speed(kind, (40, 40), true),
            );
            let silt = (
                aggregate_speed(kind, (38, 40), false),
                aggregate_speed(kind, (38, 40), true),
            );
            let dry_lane = (
                aggregate_speed(kind, (64, 49), false),
                aggregate_speed(kind, (64, 49), true),
            );
            let flooded_lane = (
                aggregate_speed(kind, (64, 79), false),
                aggregate_speed(kind, (64, 79), true),
            );
            for (name, (base, surged)) in [
                ("salt", salt),
                ("silt", silt),
                ("dry lane", dry_lane),
                ("flooded lane", flooded_lane),
            ] {
                assert!(base > 0, "{name} base speed must advance");
                assert_eq!(
                    surged * 2,
                    base * 3,
                    "{kind:?}/{name} Surge is not exactly 150%"
                );
            }
            assert!(silt.0 < salt.0, "Silt penalty must remain visible");
            assert!(
                flooded_lane.0 < dry_lane.0,
                "flooded lane penalty must remain visible"
            );
            let base_cross = u128::from(flooded_lane.0) * u128::from(dry_lane.1);
            let surge_cross = u128::from(flooded_lane.1) * u128::from(dry_lane.0);
            assert_eq!(
                base_cross, surge_cross,
                "Surge must preserve the dry/flooded lane ratio"
            );
        }
    }

    #[test]
    fn malformed_depth_files_reject_through_public_load_and_replay_paths() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir();
        let valid_replay = root.join(format!("brinewake-depth-valid-{stamp}.replay"));
        let bad_facing = root.join(format!("brinewake-depth-bad-facing-{stamp}.replay"));
        let bad_surge = root.join(format!("brinewake-depth-bad-surge-{stamp}.replay"));
        let bad_research = root.join(format!("brinewake-depth-bad-research-{stamp}.replay"));
        let valid_save = root.join(format!("brinewake-depth-valid-{stamp}.save"));
        let bad_save = root.join(format!("brinewake-depth-bad-{stamp}.save"));

        let mut world = World::new(716, Faction::Union);
        world.ai_enabled = false;
        let _worker = unit(&mut world, 0, Kind::Hook, (40, 40));
        fixture(&mut world);
        world.export_replay(&valid_replay).expect("valid replay");
        let replay_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&valid_replay).expect("read replay"))
                .expect("parse valid replay");

        let mut facing_value = replay_value.clone();
        facing_value["initial"]["entities"][0]["facing"] = serde_json::json!(8);
        fs::write(
            &bad_facing,
            serde_json::to_vec(&facing_value).expect("serialize bad facing replay"),
        )
        .expect("write bad facing replay");
        let facing_error = World::replay(&bad_facing).expect_err("bad facing replay rejected");
        assert!(facing_error.contains("invariant"), "got {facing_error}");

        let mut surge_value = replay_value.clone();
        surge_value["initial"]["entities"][1]["surge_remaining"] =
            serde_json::json!(SURGE_TICKS + 1);
        fs::write(
            &bad_surge,
            serde_json::to_vec(&surge_value).expect("serialize bad Surge replay"),
        )
        .expect("write bad Surge replay");
        let surge_error = World::replay(&bad_surge).expect_err("bad Surge replay rejected");
        assert!(surge_error.contains("invariant"), "got {surge_error}");

        let mut research_value = replay_value.clone();
        research_value["initial"]["players"][0]["research"] = serde_json::json!({
            "building": 1,
            "doctrine": "Hauling",
            "remaining": 0
        });
        fs::write(
            &bad_research,
            serde_json::to_vec(&research_value).expect("serialize bad research replay"),
        )
        .expect("write bad research replay");
        let research_error =
            World::replay(&bad_research).expect_err("bad research replay rejected");
        assert!(research_error.contains("research"), "got {research_error}");

        world.save(&valid_save).expect("valid save");
        let mut save_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&valid_save).expect("read save"))
                .expect("parse valid save");
        let mut bad_world_value = save_value["world"].clone();
        bad_world_value["entities"][0]["facing"] = serde_json::json!(8);
        let bad_world: World =
            serde_json::from_value(bad_world_value.clone()).expect("parse mutated world");
        let payload = serde_json::to_vec(&bad_world).expect("serialize mutated payload");
        save_value["world"] = bad_world_value;
        save_value["payload_digest"] =
            serde_json::json!(blake3::hash(&payload).to_hex().to_string());
        save_value["state_hash"] = serde_json::json!(bad_world.state_hash());
        fs::write(
            &bad_save,
            serde_json::to_vec(&save_value).expect("serialize bad save"),
        )
        .expect("write bad save");
        let save_error = World::load(&bad_save).expect_err("bad save rejected by invariants");
        assert!(save_error.contains("invariant"), "got {save_error}");

        assert!(valid_replay.exists(), "valid replay was preserved");
        assert!(valid_save.exists(), "valid save was preserved");
        for path in [
            valid_replay,
            bad_facing,
            bad_surge,
            bad_research,
            valid_save,
            bad_save,
        ] {
            let _ = fs::remove_file(path);
        }
    }
    #[test]
    fn a_firing_loom_is_lit_for_the_side_its_shell_falls_on() {
        let mut world = World::new(702, Faction::Union);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        // Rules 21: a Siege Loom reaches eight cells, a Riveter's sight, so
        // the unseen target is a worker, which sees seven.
        let victim = unit(&mut world, 1, Kind::Hook, (48, 64));
        fixture(&mut world);
        world.players[0].upgrades = vec![Upgrade::Siege];
        if let Some(entity) = world.entity_mut(loom) {
            entity.deployed = true;
        }
        let loom_pos = world.entity(loom).expect("loom").pos;
        assert!(
            !world.visible(1, loom_pos),
            "a Siege Loom fires from past a worker's sight"
        );
        // A deployed Loom fires on its own at the first enemy in reach.
        world.step();
        assert_eq!(world.artillery.len(), 1, "the Loom fired");
        assert!(
            world.visible(1, loom_pos),
            "the shot lights the Loom for the side it falls on"
        );
        assert!(world.beacons.iter().any(|beacon| {
            beacon.owner == 1 && beacon.pos == loom_pos && beacon.radius == LOOM_FIRE_REVEAL_RADIUS
        }));
        assert!(
            !world.beacons.iter().any(|beacon| beacon.owner == 0),
            "the firing side gains no light"
        );
        // Out of reach, the Loom stops firing and its light fades.
        if let Some(entity) = world.entity_mut(victim) {
            entity.pos = Pos::cell(50, 30);
        }
        world.clear_order(loom);
        for _ in 0..=LOOM_FIRE_REVEAL_TICKS {
            world.step();
        }
        assert!(!world.visible(1, loom_pos), "the light lasts three seconds");
    }

    #[test]
    fn a_heliostat_beam_builds_on_one_target_and_starts_again_on_another() {
        let mut world = World::new(702, Faction::Compact);
        let heliostat = unit(&mut world, 0, Kind::Heliostat, (40, 64));
        let first = unit(&mut world, 1, Kind::Bulwark, (45, 64));
        fixture(&mut world);
        if let Some(entity) = world.entity_mut(first) {
            entity.hp = 5000;
        }
        let shots = |world: &World| -> Vec<(u32, i32)> {
            world
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Shot && e.entity == Some(heliostat))
                .map(|e| (e.other.unwrap_or(0), e.amount))
                .collect()
        };
        // Packed, it holds its fire.
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![heliostat],
                    target: first,
                },
            )
            .expect("attack order");
        let mut fired = Vec::new();
        for _ in 0..30 {
            world.step();
            fired.extend(shots(&world));
        }
        assert!(fired.is_empty(), "a packed Heliostat does not fire");
        if let Some(entity) = world.entity_mut(heliostat) {
            entity.deployed = true;
        }
        for _ in 0..12 * 12 {
            world.step();
            fired.extend(shots(&world));
        }
        let amounts: Vec<i32> = fired.iter().map(|(_, amount)| *amount).collect();
        assert_eq!(&amounts[..4], &[2, 3, 4, 5], "each shot adds one");
        assert!(amounts.iter().all(|a| *a <= bw_content::HELIOSTAT_BEAM_MAX));
        assert_eq!(*amounts.last().unwrap(), bw_content::HELIOSTAT_BEAM_MAX);
        let pos = world.entity(heliostat).unwrap().pos;
        assert!(
            world.visible(1, pos),
            "the beam lights the Heliostat for the side it burns"
        );
        // A new target starts again from the first shot's damage.
        let second = unit(&mut world, 1, Kind::Reedguard, (44, 64));
        if let Some(entity) = world.entity_mut(first) {
            entity.pos = Pos::cell(80, 30);
        }
        world
            .issue(
                0,
                Command::Attack {
                    units: vec![heliostat],
                    target: second,
                },
            )
            .expect("attack the second");
        let mut next = Vec::new();
        for _ in 0..30 {
            world.step();
            next.extend(shots(&world));
        }
        assert_eq!(next.first(), Some(&(second, 2)), "a new target starts at 2");
    }

    #[test]
    fn a_deployed_heliostat_fires_on_its_own_at_what_comes_in_range() {
        let mut world = World::new(9, Faction::Compact);
        fixture(&mut world);
        let heliostat = unit(&mut world, 0, Kind::Heliostat, (30, 62));
        let riveter = unit(&mut world, 1, Kind::Riveter, (35, 62));
        if let Some(entity) = world.entity_mut(riveter) {
            entity.order = Order::Hold;
        }
        let fired = |world: &World| {
            world
                .events
                .iter()
                .any(|e| e.kind == EventKind::Shot && e.entity == Some(heliostat))
        };
        let mut shots = 0;
        for _ in 0..60 {
            world.step();
            shots += usize::from(fired(&world));
        }
        assert_eq!(shots, 0, "packed, it holds its fire");
        world.entity_mut(heliostat).unwrap().deployed = true;
        for order in [Order::Idle, Order::Hold] {
            world.entity_mut(heliostat).unwrap().order = order.clone();
            let mut shots = 0;
            for _ in 0..60 {
                world.step();
                shots += usize::from(fired(&world));
            }
            assert!(shots >= 4, "deployed on {order:?}, it fired {shots} times");
        }
    }

    #[test]
    fn glint_lights_a_long_thin_ray_toward_its_point() {
        let mut world = World::new(9, Faction::Compact);
        fixture(&mut world);
        let glinter = unit(&mut world, 0, Kind::Glinter, (30, 64));
        world.players[0].pressure = 100;
        let along = Pos::cell(46, 64);
        let beside = Pos::cell(44, 68);
        let beyond = Pos::cell(52, 64);
        assert!(!world.visible(0, along), "sixteen cells out is dark");
        // The point is close, the ray still runs its full length.
        world
            .issue(
                0,
                Command::Glint {
                    unit: glinter,
                    target: Pos::cell(33, 64),
                },
            )
            .expect("GLINT");
        world.step();
        // Twenty pressure, and at most a point of income in the same tick.
        let left = world.players[0].pressure;
        assert!((80..=81).contains(&left), "GLINT costs 20, {left} left");
        assert!(world.visible(0, along), "the ray reaches sixteen cells");
        assert!(!world.visible(0, beside), "four cells off the ray is dark");
        assert!(!world.visible(0, beyond), "past eighteen cells is dark");
        for _ in 0..bw_content::GLINT_TICKS {
            world.step();
        }
        assert!(!world.visible(0, along), "the ray lasts five seconds");
        // Only a Glinter glints.
        let other = unit(&mut world, 0, Kind::Brander, (30, 66));
        assert!(
            world
                .issue(
                    0,
                    Command::Glint {
                        unit: other,
                        target: along,
                    },
                )
                .is_err()
        );
    }

    /// A horizontal run of tidal water with dry walkable banks at both
    /// ends: (west bank x, east bank x, y).
    fn lane_crossing(world: &World) -> (i32, i32, i32) {
        let (width, height) = (i32::from(world.map.width), i32::from(world.map.height));
        let bank = |x: i32, y: i32| {
            let terrain = world.map.terrain(x, y);
            terrain.walkable() && terrain.tidal_arm().is_none()
        };
        for y in 2..height - 2 {
            for x in 1..width - 1 {
                if !bank(x, y) || world.map.terrain(x + 1, y).tidal_arm().is_none() {
                    continue;
                }
                let mut end = x + 1;
                while end < width && world.map.terrain(end, y).tidal_arm().is_some() {
                    end += 1;
                }
                if end < width
                    && end - x > 3
                    && end - x <= bw_content::LAY_MAX_ROWS as i32 + 1
                    && bank(end, y)
                    && bank(x - 2, y)
                    && bank(x - 1, y - 1)
                {
                    return (x, end, y);
                }
            }
        }
        panic!("no lane crossing on the map");
    }

    #[test]
    fn a_salter_crusts_a_causeway_that_melts_when_the_tide_moves() {
        let mut world = World::new(9, Faction::Compact);
        fixture(&mut world);
        let (west, east, y) = lane_crossing(&world);
        world.gate.tide = Tide::Flood;
        world.players[0].pressure = 200;
        let salter = unit(&mut world, 0, Kind::Salter, (west - 2, y));
        let walker = unit(&mut world, 0, Kind::Brander, (west - 1, y - 1));
        assert_eq!(world.depth_at(west + 1, y), Some(Depth::Deep));
        // Only tidal water can be laid on, and only a Salter lays.
        assert!(
            world
                .issue(
                    0,
                    Command::Lay {
                        unit: salter,
                        target: Pos::cell(west - 1, y),
                    },
                )
                .is_err()
        );
        assert!(
            world
                .issue(
                    0,
                    Command::Lay {
                        unit: walker,
                        target: Pos::cell(east - 1, y),
                    },
                )
                .is_err()
        );
        world
            .issue(
                0,
                Command::Lay {
                    unit: salter,
                    target: Pos::cell(east - 1, y),
                },
            )
            .expect("LAY");
        let rows = east - west - 1;
        let mut laid = 0;
        for _ in 0..(bw_content::LAY_ROW_TICKS as i32 * (rows + 20)) {
            if laid > 0 && world.entity(salter).unwrap().order == Order::Idle {
                break;
            }
            world.step();
            laid += world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Laid)
                .count() as i32;
        }
        assert_eq!(laid, rows, "one row per tidal cell across");
        for x in west + 1..east {
            assert_eq!(world.depth_at(x, y), Some(Depth::Dry), "crust at {x}");
        }
        assert_eq!(world.entity(salter).unwrap().order, Order::Idle);
        assert_eq!(world.entity(salter).unwrap().pos.cell_xy(), (east - 1, y));
        let left = world.players[0].pressure;
        assert!(
            left <= 200 - rows as u32 * bw_content::LAY_PRESSURE_PER_ROW + 10,
            "each row costs pressure, {left} left"
        );
        // Anyone may cross the crust while it lasts.
        world
            .issue(
                0,
                Command::Move {
                    units: vec![walker],
                    target: Pos::cell(east + 1, y),
                    queued: false,
                },
            )
            .expect("move over");
        for _ in 0..(TICK_HZ as u32 * 20) {
            world.step();
        }
        assert_eq!(
            world.entity(walker).unwrap().pos.cell_xy(),
            (east + 1, y),
            "the Brander walked the causeway"
        );
        // The tide moves and the crust melts.
        world.tide_changed("test");
        assert!(world.crust.is_empty());
        assert_eq!(world.depth_at(west + 1, y), Some(Depth::Deep));
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::CrustMelted)
        );
    }

    #[test]
    fn a_deployed_pan_boils_pressure_on_dry_ground_only() {
        let mut world = World::new(9, Faction::Compact);
        fixture(&mut world);
        let hq_pos = world.entity(hq(&world, 0)).unwrap().pos.cell_xy();
        let pan = unit(&mut world, 0, Kind::Pan, (hq_pos.0 + 6, hq_pos.1 + 6));
        let mut plain = world.clone();
        world.players[0].pressure = 0;
        plain.players[0].pressure = 0;
        world
            .issue(0, Command::Deploy { units: vec![pan] })
            .expect("deploy the Pan");
        // A second to deploy, then a minute of boiling.
        for _ in 0..(TICK_HZ as u32 + 60 * TICK_HZ as u32) {
            world.step();
            plain.step();
        }
        assert!(world.entity(pan).unwrap().deployed);
        let gained = world.players[0].pressure - plain.players[0].pressure;
        assert!(
            (39..=41).contains(&gained),
            "a Pan boils 40 a minute, gained {gained}"
        );
        // On a tidal lane it will not deploy.
        let lane = (0..128)
            .flat_map(|y| (0..128).map(move |x| (x, y)))
            .find(|&(x, y)| world.map.terrain(x, y) == Terrain::Lane0)
            .expect("a lane cell");
        let wet = unit(&mut world, 0, Kind::Pan, lane);
        world
            .issue(0, Command::Deploy { units: vec![wet] })
            .expect("the order is taken");
        for _ in 0..60 {
            world.step();
        }
        assert!(!world.entity(wet).unwrap().deployed, "no Pan on the tide");
    }

    #[test]
    fn workers_hit_at_a_wreck_leave_it_and_new_workers_do_not_walk_into_it() {
        let mut world = World::new(8, Faction::Union);
        world.ai_enabled = false;
        let hq = hq(&world, 0);
        let hq_pos = world.entity(hq).unwrap().pos;
        let workers: Vec<u32> = world
            .entities
            .iter()
            .filter(|entity| entity.owner == 0 && entity.kind == Kind::Hook)
            .map(|entity| entity.id)
            .collect();
        let Order::Gather { resource } = world.entity(workers[0]).unwrap().order.clone() else {
            panic!("starting workers gather");
        };
        let wreck_pos = world
            .map
            .resources
            .iter()
            .find(|item| item.id == resource)
            .unwrap()
            .pos;
        // Let the workers reach the wreck, then bring a raider onto it.
        for _ in 0..240 {
            world.step();
        }
        let (wx, wy) = wreck_pos.cell_xy();
        let raider = world.spawn_for_tests(1, Kind::Skipper, Pos::cell(wx + 1, wy + 1));
        let target = workers
            .iter()
            .copied()
            .min_by_key(|id| world.entity(*id).unwrap().pos.distance_sq(wreck_pos))
            .unwrap();
        world
            .issue(
                1,
                Command::Attack {
                    units: vec![raider],
                    target,
                },
            )
            .expect("the raider may attack a worker");
        let mut fled = None;
        for _ in 0..120 {
            world.step();
            if let Some(event) = world
                .events
                .iter()
                .find(|event| event.kind == EventKind::WorkersFled)
            {
                fled = Some(event.clone());
                break;
            }
        }
        let fled = fled.expect("hit workers leave the wreck");
        assert_eq!(fled.player, Some(0));
        assert!(fled.amount >= 1);
        assert!(world.in_worker_danger(0, wreck_pos), "the wreck is marked");
        assert!(
            !world.in_worker_danger(1, wreck_pos),
            "only for the side hit"
        );
        let mark = world.players[0].worker_danger[0];
        assert_eq!(mark.until, fled.tick + u64::from(WORKER_DANGER_TICKS));
        for id in &workers {
            let Some(worker) = world.entity(*id) else {
                continue;
            };
            assert_ne!(
                worker.order,
                Order::Gather { resource },
                "no worker still gathers the wreck under fire"
            );
        }
        // No worker picks the marked wreck on its own, and a rally on it
        // sends a new worker to a safe wreck instead.
        assert_ne!(
            world.nearest_salvage(0, Kind::Hook, wreck_pos, 2, true),
            Some(resource)
        );
        let rallied = world.spawn_order(0, Kind::Hook, hq_pos, Some(wreck_pos));
        assert_ne!(rallied, Order::Gather { resource });
        assert!(!matches!(rallied, Order::Move { .. }));
        // An order still sends a worker back in.
        world.entities.retain(|entity| entity.id != raider);
        world
            .issue(
                0,
                Command::Gather {
                    units: vec![workers[0]],
                    resource,
                },
            )
            .expect("a worker may be ordered back");
        world.step();
        let order = world.entity(workers[0]).map(|entity| entity.order.clone());
        assert_eq!(
            order,
            Some(Order::Gather { resource }),
            "the player's order stands until the next hit"
        );
        // The mark lapses after twenty seconds.
        world.tick = mark.until;
        assert!(!world.in_worker_danger(0, wreck_pos));
    }
}

/// Rules 14: the unit orders and controls asked for in the eighth trial.
#[cfg(test)]
mod rules14_tests {
    use super::*;
    use bw_content::{SURGE_PRESSURE, SURGE_TICKS};

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

    /// A free site for `kind` near `cell`.
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

    #[test]
    fn a_bulwark_on_an_attack_move_turns_and_deploys_at_its_reach() {
        let mut world = fixture(Faction::Union);
        let bulwark = unit(&mut world, 0, Kind::Bulwark, (40, 64));
        let enemy = unit(&mut world, 1, Kind::Reedguard, (47, 64));
        if let Some(e) = world.entity_mut(enemy) {
            e.order = Order::Hold;
        }
        world
            .issue(
                0,
                Command::AttackMove {
                    units: vec![bulwark],
                    target: Pos::cell(52, 64),
                    queued: false,
                },
            )
            .expect("attack-move");
        let mut deployed_at = None;
        for _ in 0..300 {
            world.step();
            let b = world.entity(bulwark).expect("bulwark");
            if b.deployed {
                deployed_at = Some(b.pos);
                break;
            }
        }
        let at = deployed_at.expect("the Bulwark deployed on its own");
        let enemy_pos = world.entity(enemy).expect("enemy").pos;
        let range = i64::from(spec(Kind::Bulwark).range);
        assert!(
            at.distance_sq(enemy_pos) <= range.pow(2),
            "it stopped at its reach"
        );
        let b = world.entity(bulwark).expect("bulwark");
        assert_eq!(b.facing, 2, "it faces the enemy to the east");
        assert!(in_front_arc(b, enemy_pos));
    }

    #[test]
    fn a_deployed_gun_fights_before_an_attack_move_packs_it() {
        let mut world = fixture(Faction::Union);
        let bulwark = unit(&mut world, 0, Kind::Bulwark, (40, 64));
        world
            .issue(
                0,
                Command::SetDeployed {
                    units: vec![bulwark],
                    deployed: true,
                },
            )
            .expect("deploy");
        run(&mut world, 60);
        assert!(world.entity(bulwark).expect("bulwark").deployed);
        let enemy = unit(&mut world, 1, Kind::Reedguard, (43, 64));
        if let Some(e) = world.entity_mut(enemy) {
            e.order = Order::Hold;
            e.attack_cooldown = u32::MAX / 2;
        }
        world
            .issue(
                0,
                Command::AttackMove {
                    units: vec![bulwark],
                    target: Pos::cell(40, 50),
                    queued: false,
                },
            )
            .expect("attack-move");
        run(&mut world, 20);
        let b = world.entity(bulwark).expect("bulwark");
        assert!(b.deployed && b.deploy_remaining == 0, "it keeps firing");
        // The enemy gone, it packs and follows the attack-move.
        if let Some(e) = world.entity_mut(enemy) {
            e.pos = Pos::cell(100, 100);
        }
        run(&mut world, 200);
        let b = world.entity(bulwark).expect("bulwark");
        assert!(!b.deployed, "it packed once nothing was in reach");
        assert!(b.pos.y < Pos::cell(40, 60).y, "and walked north");
    }

    #[test]
    fn deploy_and_pack_are_separate_orders_that_never_toggle() {
        let mut world = fixture(Faction::Assembly);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let set = |world: &mut World, deployed| {
            world
                .issue(
                    0,
                    Command::SetDeployed {
                        units: vec![loom],
                        deployed,
                    },
                )
                .expect("deploy order");
            world.step();
        };
        set(&mut world, true);
        run(&mut world, 40);
        assert!(world.entity(loom).expect("loom").deployed);
        // A second DEPLOY leaves a deployed Loom deployed.
        set(&mut world, true);
        assert!(world.entity(loom).expect("loom").deployed);
        assert_eq!(world.entity(loom).expect("loom").deploy_remaining, 0);
        // PACK, and DEPLOY again mid-pack: it turns back to deployed.
        set(&mut world, false);
        assert!(world.entity(loom).expect("loom").deploy_remaining > 0);
        set(&mut world, true);
        run(&mut world, 40);
        assert!(
            world.entity(loom).expect("loom").deployed,
            "mid-pack DEPLOY deploys"
        );
        // A Riveter has no deploy action.
        let riveter = unit(&mut world, 0, Kind::Reedguard, (44, 64));
        assert!(
            world
                .issue(
                    0,
                    Command::SetDeployed {
                        units: vec![riveter],
                        deployed: true,
                    },
                )
                .is_err()
        );
    }

    #[test]
    fn a_kept_deployed_loom_stays_while_its_group_moves() {
        let mut world = fixture(Faction::Assembly);
        let loom = unit(&mut world, 0, Kind::Loom, (40, 64));
        let free = unit(&mut world, 0, Kind::Loom, (40, 66));
        let guard = unit(&mut world, 0, Kind::Reedguard, (41, 65));
        world
            .issue(
                0,
                Command::SetDeployed {
                    units: vec![loom, free],
                    deployed: true,
                },
            )
            .expect("deploy");
        world
            .issue(
                0,
                Command::KeepDeployed {
                    units: vec![loom],
                    keep: true,
                },
            )
            .expect("keep");
        run(&mut world, 40);
        let start = world.entity(loom).expect("loom").pos;
        world
            .issue(
                0,
                Command::AttackMove {
                    units: vec![loom, free, guard],
                    target: Pos::cell(30, 64),
                    queued: false,
                },
            )
            .expect("attack-move");
        run(&mut world, 120);
        let kept = world.entity(loom).expect("loom");
        assert!(kept.deployed, "the kept Loom stays deployed");
        assert_eq!(kept.pos, start);
        assert!(
            !world.entity(free).expect("free").deployed,
            "the other one packed"
        );
        assert_ne!(world.entity(guard).expect("guard").pos, Pos::cell(41, 65));
        // By default an attack-move still packs and goes: the order means
        // what it meant before.
        world
            .issue(
                0,
                Command::KeepDeployed {
                    units: vec![loom],
                    keep: false,
                },
            )
            .expect("unkeep");
        world
            .issue(
                0,
                Command::AttackMove {
                    units: vec![loom],
                    target: Pos::cell(30, 64),
                    queued: false,
                },
            )
            .expect("attack-move");
        run(&mut world, 60);
        assert!(!world.entity(loom).expect("loom").deployed);
    }

    #[test]
    fn a_second_site_queues_behind_the_first_and_keeps_its_builder() {
        let mut world = fixture(Faction::Union);
        let worker = unit(&mut world, 0, Kind::Hook, (24, 58));
        world.players[0].salvage = 2000;
        let first = site_near(&world, Kind::Palisade, (26, 58));
        world
            .issue(
                0,
                Command::Build {
                    worker,
                    kind: Kind::Palisade,
                    pos: first,
                    queued: false,
                },
            )
            .expect("first site");
        world.step();
        let second = site_near(&world, Kind::Palisade, (22, 62));
        world
            .issue(
                0,
                Command::Build {
                    worker,
                    kind: Kind::Palisade,
                    pos: second,
                    queued: true,
                },
            )
            .expect("second site");
        world.step();
        let site = |world: &World, pos: Pos| {
            world
                .entities
                .iter()
                .find(|e| e.kind == Kind::Palisade && e.pos == pos)
                .map(|e| e.id)
                .expect("site")
        };
        let (a, b) = (site(&world, first), site(&world, second));
        assert_eq!(world.site_builder(a), SiteBuilder::Building);
        assert_eq!(world.site_builder(b), SiteBuilder::Queued);
        for _ in 0..3000 {
            world.step();
            if world.entity(b).is_some_and(|e| e.build_remaining == 0) {
                break;
            }
        }
        assert_eq!(world.entity(a).expect("first").build_remaining, 0);
        assert_eq!(
            world.entity(b).expect("second").build_remaining,
            0,
            "the worker went on to the queued site"
        );
        // A plain order drops the queue; the site then says it has no one.
        let third = site_near(&world, Kind::Palisade, (28, 62));
        let fourth = site_near(&world, Kind::Palisade, (20, 56));
        for (pos, queued) in [(third, false), (fourth, true)] {
            world
                .issue(
                    0,
                    Command::Build {
                        worker,
                        kind: Kind::Palisade,
                        pos,
                        queued,
                    },
                )
                .expect("site");
            world.step();
        }
        world
            .issue(
                0,
                Command::Stop {
                    units: vec![worker],
                },
            )
            .expect("stop");
        world.step();
        assert_eq!(world.site_builder(site(&world, fourth)), SiteBuilder::None);
    }

    #[test]
    fn surge_skips_machines_that_cannot_and_chains_into_capture() {
        let mut world = fixture(Faction::Union);
        let gate = world.map.gate_pos.cell_xy();
        // Two free island cells a few steps from the station.
        let mut starts = Vec::new();
        for dy in -6i32..=6 {
            for dx in -6i32..=6 {
                let cell = (gate.0 + dx, gate.1 + dy);
                if dx.abs().max(dy.abs()) >= 4
                    && world.cell_walkable_for(Kind::Riveter, cell)
                    && !world.find_path(cell, gate, Kind::Riveter).is_empty()
                {
                    starts.push(cell);
                }
            }
        }
        assert!(starts.len() >= 3, "island cells near the station");
        let riveter = unit(&mut world, 0, Kind::Riveter, starts[0]);
        let caisson = unit(&mut world, 0, Kind::Caisson, starts[1]);
        world.players[0].pressure = 100;
        world
            .issue(
                0,
                Command::Surge {
                    units: vec![riveter, caisson],
                },
            )
            .expect("the able machine surges");
        world.step();
        assert_eq!(
            world.players[0].pressure,
            100 - SURGE_PRESSURE,
            "one machine paid"
        );
        assert!(world.entity(riveter).expect("riveter").surge_remaining > 0);
        assert_eq!(world.entity(caisson).expect("caisson").surge_remaining, 0);
        assert!(
            world
                .issue(
                    0,
                    Command::Surge {
                        units: vec![caisson],
                    },
                )
                .is_err(),
            "a group with no able machine is refused"
        );
        // Capture while surging, then surge while capturing.
        world
            .issue(
                0,
                Command::Capture {
                    units: vec![riveter],
                },
            )
            .expect("capture chains after the surge");
        run(&mut world, SURGE_TICKS + 1);
        let r = world.entity(riveter).expect("riveter");
        assert!(
            matches!(r.order, Order::Capture),
            "{:?} at {:?} gate {:?}",
            r.order,
            r.pos,
            world.gate.owner
        );
        let other = unit(&mut world, 0, Kind::Riveter, starts[2]);
        world
            .issue(0, Command::Capture { units: vec![other] })
            .expect("capture");
        world.step();
        world
            .issue(0, Command::Surge { units: vec![other] })
            .expect("a capturing machine can surge");
        let mut captured = false;
        for _ in 0..3000 {
            world.step();
            if world.gate.owner == Some(0) {
                captured = true;
                break;
            }
        }
        assert!(captured, "the surged machines took the station");
    }

    #[test]
    fn a_drydock_takes_a_rally_and_new_machines_leave_the_door() {
        let mut world = fixture(Faction::Union);
        let drydock = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map(|e| e.id)
            .expect("hq");
        let _ = drydock;
        let dock_site = site_near(&world, Kind::Drydock, (28, 50));
        let dock = unit(&mut world, 0, Kind::Drydock, dock_site.cell_xy());
        assert!(
            world
                .issue(
                    0,
                    Command::Rally {
                        building: dock,
                        pos: Pos::cell(30, 60),
                    },
                )
                .is_ok(),
            "the Drydock takes a rally"
        );
        let works_site = site_near(&world, Kind::Works, (24, 54));
        let works = unit(&mut world, 0, Kind::Works, works_site.cell_xy());
        world.players[0].salvage = 1000;
        world
            .issue(
                0,
                Command::Train {
                    building: works,
                    kind: Kind::Riveter,
                },
            )
            .expect("train");
        let mut machine = None;
        for _ in 0..2000 {
            world.step();
            machine = world
                .entities
                .iter()
                .find(|e| e.owner == 0 && e.kind == Kind::Riveter)
                .map(|e| e.id);
            if machine.is_some() {
                break;
            }
        }
        let machine = machine.expect("trained");
        let door = world.entity(machine).expect("machine").pos;
        run(&mut world, 150);
        let now = world.entity(machine).expect("machine").pos;
        let away = |pos: Pos| {
            let (x, y) = pos.cell_xy();
            let (bx, by) = works_site.cell_xy();
            let n = spec(Kind::Works).footprint;
            (bx - x)
                .max(x - (bx + n - 1))
                .max(by - y)
                .max(y - (by + n - 1))
        };
        assert!(
            away(now) >= away(door) + 2,
            "it walked out from the door: {door:?} to {now:?}"
        );
    }

    #[test]
    fn an_own_worker_on_a_site_steps_off_it() {
        let mut world = fixture(Faction::Union);
        world.players[0].salvage = 1000;
        let site = site_near(&world, Kind::Works, (26, 58));
        let (sx, sy) = site.cell_xy();
        let walker = unit(&mut world, 0, Kind::Hook, (sx + 1, sy + 1));
        let builder = unit(&mut world, 0, Kind::Hook, (sx - 3, sy));
        let enemy_pos = site_near(&world, Kind::Works, (26, 66));
        let (ex, ey) = enemy_pos.cell_xy();
        let _enemy = unit(&mut world, 1, Kind::Reedguard, (ex + 1, ey + 1));
        assert!(
            world
                .issue(
                    0,
                    Command::Build {
                        worker: builder,
                        kind: Kind::Works,
                        pos: enemy_pos,
                        queued: false,
                    },
                )
                .is_err(),
            "an enemy still refuses a site"
        );
        world
            .issue(
                0,
                Command::Build {
                    worker: builder,
                    kind: Kind::Works,
                    pos: site,
                    queued: false,
                },
            )
            .expect("an own worker does not refuse the site");
        world.step();
        let (wx, wy) = world.entity(walker).expect("walker").pos.cell_xy();
        let n = spec(Kind::Works).footprint;
        assert!(
            !(wx >= sx && wx < sx + n && wy >= sy && wy < sy + n),
            "the worker stepped off the footprint"
        );
    }

    #[test]
    fn machines_standing_on_one_another_step_apart() {
        let mut world = fixture(Faction::Union);
        let ids: Vec<u32> = (0..3)
            .map(|_| unit(&mut world, 0, Kind::Riveter, (40, 64)))
            .collect();
        run(&mut world, 60);
        let near = i64::from(FP * 3 / 4).pow(2);
        for a in &ids {
            for b in &ids {
                if a < b {
                    let pa = world.entity(*a).expect("a").pos;
                    let pb = world.entity(*b).expect("b").pos;
                    assert!(
                        pa.distance_sq(pb) >= near,
                        "{pa:?} and {pb:?} still overlap"
                    );
                }
            }
        }
        assert_eq!(
            world.entity(ids[0]).expect("lowest").pos,
            Pos::cell(40, 64),
            "the lowest id stays put"
        );
    }
}
