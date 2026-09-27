//! Device-independent types and pixel-grid transforms shared by the engine.
use serde::{Deserialize, Serialize};

pub const TICK_HZ: u64 = 30;
pub const FP: i32 = 256;
pub const CANVAS_W: u32 = 640;
pub const CANVAS_H: u32 = 360;
pub const WORLD_TOP: i32 = 24;
pub const WORLD_BOTTOM: i32 = 288;

pub type EntityId = u32;
pub type PlayerId = u8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
impl Pos {
    pub const fn raw(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub const fn cell(x: i32, y: i32) -> Self {
        Self {
            x: x * FP + FP / 2,
            y: y * FP + FP / 2,
        }
    }
    pub fn cell_xy(self) -> (i32, i32) {
        (self.x.div_euclid(FP), self.y.div_euclid(FP))
    }
    pub fn distance_sq(self, other: Self) -> i64 {
        let dx = i64::from(self.x) - i64::from(other.x);
        let dy = i64::from(self.y) - i64::from(other.y);
        // Validated map coordinates are small; saturate only for defensive
        // distance queries on untrusted extreme inputs before validation.
        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
    }
    pub fn valid(self, w: u16, h: u16) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < i32::from(w) * FP && self.y < i32::from(h) * FP
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    #[default]
    Union,
    Assembly,
    /// The Saltglass Compact: fast, fragile machines on stilts.
    Compact,
}
impl Faction {
    /// Every playable faction, in menu order.
    pub const ALL: [Faction; 3] = [Faction::Union, Faction::Assembly, Faction::Compact];

    pub fn name(self) -> &'static str {
        match self {
            Self::Union => "BREAKWATER UNION",
            Self::Assembly => "SILT ASSEMBLY",
            Self::Compact => "SALTGLASS COMPACT",
        }
    }
    pub fn worker(self) -> Kind {
        match self {
            Self::Union => Kind::Hook,
            Self::Assembly => Kind::Wick,
            Self::Compact => Kind::Raker,
        }
    }
    /// The side's scout: trained at the headquarters and the Drydock.
    pub fn scout(self) -> Kind {
        match self {
            Self::Union => Kind::Tidewatch,
            Self::Assembly => Kind::Lampwright,
            Self::Compact => Kind::Stilt,
        }
    }
    pub fn army(self) -> [Kind; 3] {
        match self {
            Self::Union => [Kind::Riveter, Kind::Bulwark, Kind::Sounder],
            Self::Assembly => [Kind::Reedguard, Kind::Skipper, Kind::Loom],
            Self::Compact => [Kind::Brander, Kind::Heliostat, Kind::Glinter],
        }
    }
    /// The tier-2 roles trained at the Drydock: a scout, a repair machine,
    /// the faction's answer to the water, and its transport (the Compact,
    /// which has none, trains the Pan in that slot).
    pub fn drydock_roles(self) -> [Kind; 4] {
        match self {
            Self::Union => [Kind::Tidewatch, Kind::Caulker, Kind::Caisson, Kind::Lifter],
            Self::Assembly => [Kind::Lampwright, Kind::Tender, Kind::Dredger, Kind::Barge],
            Self::Compact => [Kind::Stilt, Kind::Glazier, Kind::Salter, Kind::Pan],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Hook,
    Riveter,
    Bulwark,
    Sounder,
    Wick,
    Skipper,
    Reedguard,
    Loom,
    Headquarters,
    Works,
    Dropoff,
    Condenser,
    Tower,
    Tidewatch,
    Caulker,
    Caisson,
    Lampwright,
    Tender,
    Dredger,
    Drydock,
    Palisade,
    /// The Assembly's transport: a hull for deep and shallow water.
    Barge,
    /// The Union's transport: a lifter that flies over everything.
    Lifter,
    /// The Compact's worker.
    Raker,
    /// The Compact's fast line machine: a short fire-lance.
    Brander,
    /// The Compact's deploying heavy: a mirror beam that builds on one
    /// target.
    Heliostat,
    /// The Compact's harasser, which carries GLINT.
    Glinter,
    /// The Compact's scout.
    Stilt,
    /// The Compact's mender.
    Glazier,
    /// Lays salt causeways over tidal water.
    Salter,
    /// Deploys on dry ground to boil pressure.
    Pan,
}
impl Kind {
    /// Machines that carry others: no gun, a hold of four.
    pub fn is_transport(self) -> bool {
        matches!(self, Self::Barge | Self::Lifter)
    }
    pub fn is_worker(self) -> bool {
        matches!(self, Self::Hook | Self::Wick | Self::Raker)
    }
    pub fn is_building(self) -> bool {
        matches!(
            self,
            Self::Headquarters
                | Self::Works
                | Self::Dropoff
                | Self::Condenser
                | Self::Tower
                | Self::Drydock
                | Self::Palisade
        )
    }
    /// Machines that gather salvage: the workers and the Dredger.
    pub fn gathers(self) -> bool {
        self.is_worker() || self == Self::Dredger
    }
    /// The scouts: no gun, a wide view, doubled on Hold.
    pub fn is_scout(self) -> bool {
        matches!(self, Self::Tidewatch | Self::Lampwright | Self::Stilt)
    }
    /// The repair machines.
    pub fn repairs(self) -> bool {
        matches!(self, Self::Caulker | Self::Tender | Self::Glazier)
    }
    /// Buildings that produce machines.
    pub fn produces(self) -> bool {
        matches!(self, Self::Headquarters | Self::Works | Self::Drydock)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Hook => "HOOK",
            Self::Riveter => "RIVETER",
            Self::Bulwark => "BULWARK",
            Self::Sounder => "SOUNDER",
            Self::Wick => "WICK",
            Self::Skipper => "SKIPPER",
            Self::Reedguard => "REEDGUARD",
            Self::Loom => "LOOM",
            Self::Headquarters => "HEADQUARTERS",
            Self::Works => "WORKS",
            Self::Dropoff => "SALVAGE YARD",
            Self::Condenser => "CONDENSER",
            Self::Tower => "DEFENSE NEST",
            Self::Tidewatch => "TIDEWATCH",
            Self::Caulker => "CAULKER",
            Self::Caisson => "CAISSON",
            Self::Lampwright => "LAMPWRIGHT",
            Self::Tender => "TENDER",
            Self::Dredger => "DREDGER",
            Self::Drydock => "DRYDOCK",
            Self::Palisade => "PALISADE",
            Self::Barge => "BARGE",
            Self::Lifter => "LIFTER",
            Self::Raker => "RAKER",
            Self::Brander => "BRANDER",
            Self::Heliostat => "HELIOSTAT",
            Self::Glinter => "GLINTER",
            Self::Stilt => "STILT",
            Self::Glazier => "GLAZIER",
            Self::Salter => "SALTER",
            Self::Pan => "PAN",
        }
    }
    pub fn asset(self, faction: Faction) -> &'static str {
        // Buildings are shared kinds with each faction's own art.
        let by_faction = |union, assembly, compact| match faction {
            Faction::Union => union,
            Faction::Assembly => assembly,
            Faction::Compact => compact,
        };
        match self {
            Self::Hook => "hook",
            Self::Riveter => "riveter",
            Self::Bulwark => "bulwark",
            Self::Sounder => "sounder",
            Self::Wick => "wick",
            Self::Skipper => "skipper",
            Self::Reedguard => "reedguard",
            Self::Loom => "loom",
            Self::Headquarters => by_faction("union_hq", "assembly_hq", "compact_hq"),
            Self::Works => by_faction("union_works", "assembly_works", "compact_works"),
            Self::Dropoff => by_faction("dropoff", "dropoff", "compact_dropoff"),
            Self::Condenser => "condenser",
            Self::Tower => by_faction("tower", "tower", "compact_tower"),
            Self::Tidewatch => "tidewatch",
            Self::Caulker => "caulker",
            Self::Caisson => "caisson",
            Self::Lampwright => "lampwright",
            Self::Tender => "tender",
            Self::Dredger => "dredger",
            Self::Drydock => by_faction("union_drydock", "assembly_drydock", "compact_drydock"),
            Self::Palisade => by_faction("union_palisade", "assembly_palisade", "compact_palisade"),
            Self::Barge => "barge",
            Self::Lifter => "lifter",
            Self::Raker => "raker",
            Self::Brander => "brander",
            Self::Heliostat => "heliostat",
            Self::Glinter => "glinter",
            Self::Stilt => "stilt",
            Self::Glazier => "glazier",
            Self::Salter => "salter",
            Self::Pan => "pan",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Movement {
    Wheel,
    Walker,
    Paddle,
    Static,
    /// A hull: deep and shallow water only.
    Hull,
    /// Flight: everything but rock, over buildings and machines.
    Air,
}

/// Ground.  The tidal lanes and rims belong to an arm of water, numbered by
/// the map: on the Split Basin arm 0 is the north and arm 1 the south, and
/// they keep those names on disk so two-arm saves and replays are unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
    Salt,
    Silt,
    Deep,
    /// The crossing across arm 0: the Split Basin's north lane.
    #[serde(rename = "NorthLane")]
    Lane0,
    /// The crossing across arm 1: the Split Basin's south lane.
    #[serde(rename = "SouthLane")]
    Lane1,
    Rock,
    /// The path around the outside of arm 1: the Split Basin's south rim.
    #[serde(rename = "SouthRim")]
    Rim1,
    /// The path around the outside of arm 0: the Split Basin's north rim.
    #[serde(rename = "NorthRim")]
    Rim0,
    /// The crossing and the rim of a third arm (the Confluence).
    Lane2,
    Rim2,
}
impl Terrain {
    /// Ground a machine may stand on at some tide; the tide decides the rest.
    pub fn walkable(self) -> bool {
        !matches!(self, Self::Deep | Self::Rock)
    }
    /// The arm of water whose tide this cell follows, for lanes and rims.
    pub fn tidal_arm(self) -> Option<u8> {
        match self {
            Self::Lane0 | Self::Rim0 => Some(0),
            Self::Lane1 | Self::Rim1 => Some(1),
            Self::Lane2 | Self::Rim2 => Some(2),
            _ => None,
        }
    }
    pub fn is_tidal(self) -> bool {
        self.tidal_arm().is_some()
    }
    /// A crossing lane, of any arm.
    pub fn is_lane(self) -> bool {
        matches!(self, Self::Lane0 | Self::Lane1 | Self::Lane2)
    }
    /// A rim path, of any arm.
    pub fn is_rim(self) -> bool {
        matches!(self, Self::Rim0 | Self::Rim1 | Self::Rim2)
    }
    /// The lane of an arm.
    pub fn lane(arm: u8) -> Self {
        match arm {
            0 => Self::Lane0,
            1 => Self::Lane1,
            _ => Self::Lane2,
        }
    }
    /// The rim of an arm.
    pub fn rim(arm: u8) -> Self {
        match arm {
            0 => Self::Rim0,
            1 => Self::Rim1,
            _ => Self::Rim2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub scale: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Viewport {
    pub fn new(w: u32, h: u32) -> Option<Self> {
        let k = (w / CANVAS_W).min(h / CANVAS_H);
        if k == 0 {
            return None;
        }
        Some(Self {
            scale: k,
            x: (w - CANVAS_W * k) / 2,
            y: (h - CANVAS_H * k) / 2,
            width: CANVAS_W * k,
            height: CANVAS_H * k,
        })
    }
    pub fn inverse(self, x: f64, y: f64) -> Option<(i32, i32)> {
        if !x.is_finite()
            || !y.is_finite()
            || x < f64::from(self.x)
            || y < f64::from(self.y)
            || x >= f64::from(self.x + self.width)
            || y >= f64::from(self.y + self.height)
        {
            return None;
        }
        Some((
            ((x - f64::from(self.x)) / f64::from(self.scale)).floor() as i32,
            ((y - f64::from(self.y)) / f64::from(self.scale)).floor() as i32,
        ))
    }
}

/// Pixel-quantized 2:1 dimetric camera. The same inverse is used for targeting.
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
}
impl Camera {
    pub fn project(self, p: Pos) -> (i32, i32) {
        (
            (p.x - p.y) * 16 / FP - self.x,
            (p.x + p.y) * 8 / FP - self.y,
        )
    }
    pub fn unproject(self, x: i32, y: i32) -> Pos {
        let sx = x + self.x;
        let sy = y + self.y;
        Pos::raw((sx + 2 * sy) * FP / 32, (2 * sy - sx) * FP / 32)
    }
    pub fn center(&mut self, p: Pos) {
        self.x = (p.x - p.y) * 16 / FP - 320;
        self.y = (p.x + p.y) * 8 / FP - 156;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_edges_and_odd_borders() {
        for (w, h, k, x, y) in [
            (1920, 1080, 3, 0, 0),
            (2560, 1440, 4, 0, 0),
            (3840, 2160, 6, 0, 0),
            (1366, 768, 2, 43, 24),
            (3440, 1440, 4, 440, 0),
            (1367, 769, 2, 43, 24),
        ] {
            let v = Viewport::new(w, h).unwrap();
            assert_eq!((v.scale, v.x, v.y), (k, x, y));
            assert_eq!(v.inverse(x as f64, y as f64), Some((0, 0)));
            assert_eq!(v.inverse((x + 640 * k) as f64, y as f64), None);
            assert_eq!(v.inverse(x as f64 - 0.1, y as f64), None);
        }
        assert!(Viewport::new(639, 360).is_none());
    }
    #[test]
    fn camera_cell_centers_round_trip() {
        let c = Camera { x: -221, y: 495 };
        for x in 0..128 {
            for y in 0..128 {
                let p = Pos::cell(x, y);
                let (sx, sy) = c.project(p);
                assert_eq!(c.unproject(sx, sy), p)
            }
        }
    }
}
