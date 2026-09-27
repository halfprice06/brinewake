//! The three-arm sluice station (art v22), drawn on maps with three arms.
//!
//! The Split Basin's station is a two-shaft lock whose drawings carry N and
//! S.  On the Confluence those letters named nothing, so it gets its own
//! station, built on the same lock: one gauge shaft per arm, standing where
//! the arm leaves the station on screen (left, right, or on the front lip
//! for the arm that runs toward the viewer).  A shaft's float shows its
//! arm's water: low when dry, high when deep, half way on the neutral tide.
//! The semaphore and the handwheel's pale pointer point along the dry arm
//! and stand upright when no arm is dry.  No letters are drawn.
//!
//! The art is in pieces (`tools/art_v22`), so any tide can be composed and
//! the change between two is seen: the semaphore swings over, the wheel
//! turns, and the floats ease to their new rows in the two seconds the
//! lane's water takes.  This module is presentation only: it reads the
//! authoritative gate and the tick, and never changes the world.

use bw_sim::{Gate, Tide, World};

/// A switch plays over twelve frames of five ticks: two seconds, as v20.
pub const FRAME_TICKS: u64 = 5;
pub const FRAMES: u64 = 12;

/// The base: hull, walls, the three shafts, mast, bar and wheel rim.
pub const BASE_KEY: &str = "station3";

/// The semaphore angles drawn, counter-clockwise from screen east.
pub const FLAG_ANGLES: [i32; 10] = [0, 22, 45, 90, 135, 158, 180, 225, 270, 315];
/// The semaphore when no arm is dry.
const UPRIGHT_FLAG: i32 = 90;
/// The wheel's pointer when no arm is dry, in eighths clockwise from east.
const UPRIGHT_WHEEL: u8 = 6;

/// Where a shaft stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Left,
    Right,
    Front,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Left, Slot::Right, Slot::Front];

    fn index(self) -> usize {
        match self {
            Slot::Left => 0,
            Slot::Right => 1,
            Slot::Front => 2,
        }
    }

    /// Pixels between the float's dry and deep rows.
    pub fn travel(self) -> i32 {
        match self {
            Slot::Front => 16,
            _ => 20,
        }
    }

    /// The float piece at `px` pixels above the dry row, and the offset it
    /// is drawn at: the right shaft reuses the left shaft's pieces.
    /// `px` may be one past either end (a float's overshoot).
    pub fn float_piece(self, px: i32) -> (String, i32) {
        let px = px.clamp(-1, self.travel() + 1);
        // Pieces are numbered from the overshoot below the dry row.
        let index = px + 1;
        match self {
            Slot::Front => (format!("station3_float_front_{index}"), 0),
            Slot::Left => (format!("station3_float_side_{index}"), 0),
            Slot::Right => (format!("station3_float_side_{index}"), 46),
        }
    }

    /// The warning pointer's offset from the left shaft's, aimed at the row
    /// a float `px` above dry will stand on.
    pub fn pointer_offset(self, px: i32) -> (i32, i32) {
        let dx = match self {
            Slot::Left => 0,
            Slot::Right => 46,
            Slot::Front => 20,
        };
        // The piece's tip faces the left shaft's float when deep (px 20).
        (dx, 20 - px)
    }

    /// The semaphore angle pointing along this slot's arm.
    fn flag(self) -> i32 {
        match self {
            Slot::Left => 158,
            Slot::Right => 22,
            Slot::Front => 270,
        }
    }

    /// The wheel's pointer along this slot's arm, eighths clockwise from
    /// east: up-left, up-right, down.
    fn wheel(self) -> u8 {
        match self {
            Slot::Left => 5,
            Slot::Right => 7,
            Slot::Front => 2,
        }
    }
}

/// Whether a map gets the three-arm station.
pub fn applies(world: &World) -> bool {
    world.map.layout().arms.len() == 3
}

/// The slot of each arm's shaft, in arm order: where the arm's lane leaves
/// the station on the 2:1 screen.  An arm heading down the screen more
/// than across it stands in front; the others by their side.
pub fn arm_slots(world: &World) -> Vec<Slot> {
    let layout = world.map.layout();
    let (sx, sy) = layout.station;
    layout
        .arms
        .iter()
        .map(|arm| {
            let (cx, cy) = arm.centre;
            let across = (cx - cy) - (sx - sy);
            let down = (cx + cy) - (sx + sy);
            if down > 0 && across.abs() * 2 < down {
                Slot::Front
            } else if across < 0 {
                Slot::Left
            } else {
                Slot::Right
            }
        })
        .collect()
}

/// What the station shows: each slot's water in twentieths of deep, and
/// the slot whose arm is dry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pose {
    pub levels: [u8; 3],
    pub dry: Option<Slot>,
}

impl Pose {
    const NEUTRAL: Pose = Pose {
        levels: [10; 3],
        dry: None,
    };
    const FLOOD: Pose = Pose {
        levels: [20; 3],
        dry: None,
    };

    fn open(dry_arm: usize, slots: &[Slot]) -> Pose {
        let mut levels = [20; 3];
        let dry = slots.get(dry_arm).copied();
        if let Some(slot) = dry {
            levels[slot.index()] = 0;
        }
        Pose { levels, dry }
    }

    fn flag(self) -> i32 {
        self.dry.map_or(UPRIGHT_FLAG, Slot::flag)
    }

    fn wheel(self) -> u8 {
        self.dry.map_or(UPRIGHT_WHEEL, Slot::wheel)
    }

    fn px(self, slot: Slot) -> i32 {
        px_of(i32::from(self.levels[slot.index()]), slot)
    }
}

fn px_of(twentieths: i32, slot: Slot) -> i32 {
    (twentieths * slot.travel() + 10).div_euclid(20)
}

/// The pose the gate rests in now.
pub fn rest_pose(gate: &Gate, slots: &[Slot]) -> Pose {
    match gate.tide {
        Tide::Neutral => Pose::NEUTRAL,
        Tide::Flood => Pose::FLOOD,
        Tide::Open => Pose::open(gate.dry_arm.index(), slots),
    }
}

/// The pose a running warning leads to, when the gate says which.
pub fn warning_target(gate: &Gate, slots: &[Slot]) -> Option<Pose> {
    gate.warning_until?;
    if gate.flood_pending {
        Some(Pose::FLOOD)
    } else if gate.ebb_pending {
        Some(Pose::NEUTRAL)
    } else {
        gate.switch_target.map(|arm| Pose::open(arm.index(), slots))
    }
}

/// A swing from one pose to another, started on a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub from: Pose,
    pub to: Pose,
    pub start: u64,
}

/// Float travel through a switch, in twentieths of the move: still while
/// the wheel starts, fastest mid-stroke, a one-pixel overshoot, then rest
/// (v20's lock).
const FLOAT_TRAVEL: [i32; FRAMES as usize] = [0, 0, 1, 2, 5, 8, 12, 15, 18, 20, 21, 20];
/// The semaphore leads: it lands on the new side before the water moves.
const FLAG_TRAVEL: [i32; FRAMES as usize] = [5, 10, 15, 20, 20, 20, 20, 20, 20, 20, 20, 20];

impl Motion {
    pub fn rest(pose: Pose, tick: u64) -> Motion {
        Motion {
            from: pose,
            to: pose,
            start: tick,
        }
    }

    /// Follow the authoritative pose: a new one starts a swing from the
    /// pose last shown.  A tick before the swing's start (a replay seeking
    /// back) shows the new pose at rest.
    pub fn follow(&mut self, target: Pose, tick: u64) {
        if tick < self.start {
            *self = Motion::rest(target, tick);
        } else if target != self.to {
            *self = Motion {
                from: self.to,
                to: target,
                start: tick,
            };
        }
    }

    /// The switch frame at `tick`, or None once the swing has landed.
    fn frame(&self, tick: u64) -> Option<usize> {
        if self.from == self.to {
            return None;
        }
        let frame = tick.checked_sub(self.start)? / FRAME_TICKS;
        (frame < FRAMES).then_some(frame as usize)
    }
}

/// The pieces to draw this tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    /// Each slot's float, in pixels above its dry row.
    pub floats: [(Slot, i32); 3],
    pub flag: i32,
    pub wheel: u8,
    /// Warning pointers: the slot, the float row they aim at, the pulse.
    pub pointers: Vec<(Slot, i32, u8)>,
}

/// What the station shows at `tick`: the swing if one is running, else
/// the rest pose, with the warning's pointers and wheel rock when the gate
/// is counting down (`phase` is the gate's warning phase, 0..4).
pub fn drawn(motion: &Motion, gate: &Gate, slots: &[Slot], tick: u64, phase: Option<u8>) -> Drawn {
    let floats = Slot::ALL.map(|slot| (slot, motion.to.px(slot)));
    let mut out = Drawn {
        floats,
        flag: motion.to.flag(),
        wheel: motion.to.wheel(),
        pointers: Vec::new(),
    };
    if let Some(frame) = motion.frame(tick) {
        let (from, to) = (motion.from, motion.to);
        out.floats = Slot::ALL.map(|slot| {
            let a = i32::from(from.levels[slot.index()]);
            let b = i32::from(to.levels[slot.index()]);
            let twentieths = a * 20 + (b - a) * FLOAT_TRAVEL[frame];
            (slot, (twentieths * slot.travel() + 200).div_euclid(400))
        });
        out.flag = swing(from.flag(), to.flag(), FLAG_TRAVEL[frame]);
        out.wheel = turn(from.wheel(), to.wheel(), frame);
        return out;
    }
    if let (Some(phase), Some(target)) = (phase, warning_target(gate, slots)) {
        for slot in Slot::ALL {
            let (now, next) = (motion.to.px(slot), target.px(slot));
            if now != next {
                out.pointers.push((slot, next, phase % 2));
            }
        }
    }
    if let Some(phase) = phase {
        // The wheel rocks an eighth on alternate pulses, as on v13's lock.
        out.wheel = (out.wheel + phase % 2) % 8;
    }
    out
}

/// The semaphore's angle `twentieths` of the way from `a` to `b`, snapped
/// to a drawn angle.  A swing to or from the front arm (straight down)
/// passes the side it starts or ends on, never over the top.
fn swing(a: i32, b: i32, twentieths: i32) -> i32 {
    let down_via_right = |other: i32| other < UPRIGHT_FLAG;
    let (a, b) = match (a, b) {
        (270, other) if down_via_right(other) => (-90, other),
        (other, 270) if down_via_right(other) => (other, -90),
        pair => pair,
    };
    nearest_flag(a + (b - a) * twentieths / 20)
}

fn nearest_flag(angle: i32) -> i32 {
    let angle = angle.rem_euclid(360);
    *FLAG_ANGLES
        .iter()
        .min_by_key(|&&drawn| {
            let d = (drawn - angle).rem_euclid(360);
            d.min(360 - d)
        })
        .expect("drawn angles")
}

/// The wheel through a switch: a full turn and on to the new pointer,
/// landing on frame 9 and holding.
fn turn(from: u8, to: u8, frame: usize) -> u8 {
    let total = 8 + u32::from((to + 8 - from) % 8);
    let done = (total * (frame as u32 + 1)).div_ceil(10).min(total);
    ((u32::from(from) + done) % 8) as u8
}

/// The atlas keys to draw, in order, each with its offset from the
/// station's anchor.
pub fn pieces(drawn: &Drawn) -> Vec<(String, i32, i32)> {
    let mut out = vec![(BASE_KEY.to_string(), 0, 0)];
    for (slot, px) in drawn.floats {
        let (key, dx) = slot.float_piece(px);
        out.push((key, dx, 0));
    }
    out.push((format!("station3_flag_{}", drawn.flag), 0, 0));
    out.push((format!("station3_wheel_{}", drawn.wheel), 0, 0));
    for &(slot, px, pulse) in &drawn.pointers {
        let (dx, dy) = slot.pointer_offset(px);
        out.push((format!("station3_pointer_{pulse}"), dx, dy));
    }
    out
}

/// The key whose silhouette stands for the station on this map, for the
/// capture bar's height and the pointer's hit test: the three-arm base, or
/// the Split Basin's current lock drawing.
pub fn station_key(world: &World) -> String {
    if applies(world) {
        return BASE_KEY.to_string();
    }
    crate::presentation::gate_asset(
        world.gate.north_dry(),
        world.gate.warning_until.is_some(),
        crate::presentation::gate_warning_phase(world.gate.warning_until, world.tick).unwrap_or(0),
    )
}

impl crate::game::Game {
    /// This frame's three-arm station pieces (key and offset from the
    /// station's anchor), advancing its swing; None on a two-arm map or
    /// when the atlas lacks the art, where the lock drawing is used.
    pub(crate) fn station3_pieces(&mut self) -> Option<Vec<(String, i32, i32)>> {
        if !applies(&self.world)
            || !self
                .atlas
                .as_ref()
                .is_some_and(|a| a.sprites.contains_key(BASE_KEY))
        {
            return None;
        }
        let slots = arm_slots(&self.world);
        let tick = self.world.tick;
        let target = rest_pose(&self.world.gate, &slots);
        let motion = self.station3.get_or_insert(Motion::rest(target, tick));
        motion.follow(target, tick);
        let motion = *motion;
        let phase = crate::presentation::gate_warning_phase(self.world.gate.warning_until, tick);
        Some(pieces(&drawn(
            &motion,
            &self.world.gate,
            &slots,
            tick,
            phase,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;
    use bw_sim::{Arm, MapId};

    fn confluence() -> World {
        World::with_map(
            310,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("confluence")
    }

    #[test]
    fn the_confluence_arms_stand_where_they_leave_the_station() {
        let world = confluence();
        assert!(applies(&world));
        // E runs up-right, W up-left, S down toward the viewer.
        assert_eq!(
            arm_slots(&world),
            vec![Slot::Right, Slot::Left, Slot::Front]
        );
        assert!(!applies(&World::new(310, Faction::Union)));
    }

    #[test]
    fn each_tide_rests_on_its_own_drawing() {
        let world = confluence();
        let slots = arm_slots(&world);
        let mut gate = world.gate.clone();
        gate.tide = Tide::Neutral;
        let neutral = rest_pose(&gate, &slots);
        assert_eq!(neutral.levels, [10; 3]);
        assert_eq!((neutral.flag(), neutral.wheel()), (90, 6));
        gate.tide = Tide::Open;
        for (arm, slot, flag) in [
            (0, Slot::Right, 22),
            (1, Slot::Left, 158),
            (2, Slot::Front, 270),
        ] {
            gate.dry_arm = Arm(arm);
            let pose = rest_pose(&gate, &slots);
            assert_eq!(pose.dry, Some(slot));
            assert_eq!(pose.flag(), flag);
            assert_eq!(pose.px(slot), 0, "the dry arm's float sits low");
            for other in Slot::ALL.into_iter().filter(|&s| s != slot) {
                assert_eq!(pose.px(other), other.travel(), "deep arms float high");
            }
        }
        gate.tide = Tide::Flood;
        assert_eq!(rest_pose(&gate, &slots).levels, [20; 3]);
    }

    #[test]
    fn a_switch_swings_the_flag_first_then_the_floats_and_lands_on_the_rest_drawing() {
        let world = confluence();
        let slots = arm_slots(&world);
        let w_dry = Pose::open(1, &slots);
        let s_dry = Pose::open(2, &slots);
        let mut motion = Motion::rest(w_dry, 0);
        motion.follow(s_dry, 100);
        let gate = world.gate.clone();
        let at = |tick| drawn(&motion, &gate, &slots, tick, None);
        // The first frame: the flag has left the W side toward the front,
        // passing the left (never over the top); the floats have not moved.
        let first = at(100);
        assert!((158..=270).contains(&first.flag), "{}", first.flag);
        assert_eq!(first.floats[0], (Slot::Left, 0));
        // Mid-stroke both floats are travelling.
        let mid = at(100 + 6 * FRAME_TICKS);
        assert!(mid.floats[0].1 > 0 && mid.floats[0].1 < 20);
        assert!(mid.floats[2].1 > 0 && mid.floats[2].1 < 16);
        // Frame 10 overshoots a pixel, frame 11 is home.
        assert_eq!(at(100 + 10 * FRAME_TICKS).floats[0].1, 21);
        let landed = at(100 + FRAMES * FRAME_TICKS);
        assert_eq!(
            landed,
            drawn(&Motion::rest(s_dry, 0), &gate, &slots, 0, None)
        );
        assert_eq!(at(100 + 11 * FRAME_TICKS).wheel, landed.wheel);
        // A replay that seeks back before the swing shows the pose at rest.
        motion.follow(s_dry, 50);
        assert_eq!(motion, Motion::rest(s_dry, 50));
    }

    #[test]
    fn the_flag_swings_to_the_front_by_the_nearer_side() {
        assert_eq!(swing(22, 270, 10), 315);
        assert_eq!(swing(158, 270, 10), 225);
        assert_eq!(swing(270, 22, 20), 22);
        assert_eq!(swing(158, 22, 10), 90);
        for (a, b) in [(22, 270), (158, 270), (90, 270), (270, 90), (158, 22)] {
            for t in 0..=20 {
                assert!(FLAG_ANGLES.contains(&swing(a, b, t)));
            }
        }
    }

    #[test]
    fn the_wheel_turns_a_full_turn_and_lands_on_the_new_pointer() {
        for from in 0..8 {
            for to in 0..8 {
                assert_eq!(turn(from, to, 9), to);
                assert_eq!(turn(from, to, 11), to);
                assert_ne!(turn(from, to, 3), from, "the wheel is turning mid-switch");
            }
        }
    }

    fn atlas() -> crate::canvas::Atlas {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        crate::canvas::Atlas::load(&base).expect("atlas")
    }

    #[test]
    fn every_piece_the_station_can_draw_is_in_the_atlas() {
        let atlas = atlas();
        let mut keys = vec![BASE_KEY.to_string()];
        for slot in Slot::ALL {
            for px in -1..=slot.travel() + 1 {
                keys.push(slot.float_piece(px).0);
            }
        }
        keys.extend(FLAG_ANGLES.iter().map(|a| format!("station3_flag_{a}")));
        keys.extend((0..8).map(|s| format!("station3_wheel_{s}")));
        keys.extend((0..2).map(|p| format!("station3_pointer_{p}")));
        for key in keys {
            assert!(atlas.sprites.contains_key(&key), "{key} missing");
        }
    }

    #[test]
    fn the_station_stands_on_its_masonry_with_no_shadow_beneath() {
        // CONTRIBUTING.md: no shadow under buildings or the sluice.  The fitted
        // masonry reaches the anchor; nothing is painted below it, where
        // the lock's old cast shadow lay.
        let atlas = atlas();
        assert!(atlas.hit(BASE_KEY, 0, 4), "masonry slab missing");
        for dx in -60..=60 {
            for dy in 12..16 {
                assert!(!atlas.hit(BASE_KEY, dx, dy), "shadow at {dx},{dy}");
            }
        }
        // No piece paints below the station's lowest part, the wheel.
        let world = confluence();
        let slots = arm_slots(&world);
        for dry in 0..3 {
            let pose = Pose::open(dry, &slots);
            for (key, dx, dy) in
                pieces(&drawn(&Motion::rest(pose, 0), &world.gate, &slots, 0, None))
            {
                let s = &atlas.sprites[&key];
                let bottom = s.h as i32 - s.anchor_y + dy;
                assert!(
                    bottom <= 16,
                    "{key} reaches {bottom} below the anchor ({dx})"
                );
            }
        }
    }

    /// A calm match on `map` seen through the real renderer, camera on the
    /// station, at world zoom `zoom`.
    fn field(map: MapId, zoom: crate::zoom::Zoom) -> crate::game::Game {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!("bw-station3-{}", std::process::id()));
        let mut g = crate::game::Game::new_with_data_dir(base, data);
        g.resize_view(1280, 720);
        g.map = map;
        g.start();
        g.world.ai_enabled = false;
        g.world.revealed = true;
        g.message.clear();
        g.cursor = (-999, -999);
        g.zoom = zoom;
        g.camera.center(g.world.map.gate_pos);
        g
    }

    /// Put the station in `gate`'s state at rest (or in its warning) and
    /// draw the frame; returns the station anchor's window position.
    fn show(g: &mut crate::game::Game, tide: Tide, dry: u8, warn: Option<u8>) -> (i32, i32) {
        g.world.gate.tide = tide;
        g.world.gate.dry_arm = Arm(dry);
        g.world.gate.opened = tide != Tide::Neutral;
        g.world.gate.warning_until = warn.map(|_| g.world.tick + 150);
        g.world.gate.switch_target = warn.map(Arm);
        g.station3 = None;
        g.render();
        g.project(g.world.map.gate_pos)
    }

    /// The window pixels, relative to the anchor, where the lock's N and S
    /// signs are painted in cream (art v13 `LETTER_BOXES`, in scene texels).
    fn sign_pixels(g: &crate::game::Game, anchor: (i32, i32), scale: i32) -> Vec<(i32, i32)> {
        const STRIPE: [u8; 4] = [231, 217, 181, 255];
        let mut found = Vec::new();
        for (x0, y0, x1, y1) in [(50, 23, 64, 34), (96, 23, 110, 34)] {
            for y in (y0 - 128) * scale..(y1 - 128) * scale {
                for x in (x0 - 80) * scale..(x1 - 80) * scale {
                    if g.canvas.get(anchor.0 + x, anchor.1 + y) == Some(STRIPE) {
                        found.push((x, y));
                    }
                }
            }
        }
        found
    }

    /// Saves a window crop round the station when `BW_STATION3_FRAMES`
    /// names a folder (review evidence; the assertions do not need it).
    fn keep(g: &crate::game::Game, anchor: (i32, i32), scale: i32, name: &str) {
        let Some(dir) = std::env::var_os("BW_STATION3_FRAMES") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("frame folder");
        let (w, h) = (200 * scale, 176 * scale);
        let (x0, y0) = (anchor.0 - 100 * scale, anchor.1 - 144 * scale);
        let mut out = image::RgbaImage::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                if let Some(c) = g.canvas.get(x0 + x, y0 + y) {
                    out.put_pixel(x as u32, y as u32, image::Rgba(c));
                }
            }
        }
        out.save(dir.join(format!("{name}-{scale}x.png")))
            .expect("save crop");
    }

    #[test]
    fn the_confluence_field_shows_the_station_without_north_or_south_signs() {
        use crate::zoom::Zoom;
        for (zoom, scale) in [(Zoom::Overview, 1), (Zoom::Wide, 2)] {
            // The detector finds the lock's signs on the Split Basin, which
            // keeps them: it has a north and a south lane.
            let mut basin = field(MapId::SplitBasin, zoom);
            let at = show(&mut basin, Tide::Open, 0, None);
            let signs = sign_pixels(&basin, at, scale);
            assert!(signs.len() > 10, "{} sign pixels on the basin", signs.len());
            assert!(basin.station3_pieces().is_none());
            keep(&basin, at, scale, "basin-north-dry");

            let mut g = field(MapId::Confluence, zoom);
            assert!(g.station3_pieces().is_some(), "three-arm art not drawn");
            let frames: [(&str, Tide, u8, Option<u8>); 7] = [
                ("confluence-neutral", Tide::Neutral, 0, None),
                ("confluence-e-dry", Tide::Open, 0, None),
                ("confluence-w-dry", Tide::Open, 1, None),
                ("confluence-s-dry", Tide::Open, 2, None),
                ("confluence-flood", Tide::Flood, 0, None),
                ("confluence-warning-w-to-s", Tide::Open, 1, Some(2)),
                ("confluence-warning-s-to-e", Tide::Open, 2, Some(0)),
            ];
            for (name, tide, dry, warn) in frames {
                let at = show(&mut g, tide, dry, warn);
                let left = sign_pixels(&g, at, scale);
                assert!(left.is_empty(), "{name}: N/S sign pixels at {left:?}");
                keep(&g, at, scale, name);
            }
            // A switch seen mid-stroke: W dry to S dry, half way.
            show(&mut g, Tide::Open, 1, None);
            g.world.gate.dry_arm = Arm(2);
            g.render();
            g.world.tick += 6 * FRAME_TICKS;
            g.render();
            let at = g.project(g.world.map.gate_pos);
            assert!(sign_pixels(&g, at, scale).is_empty());
            keep(&g, at, scale, "confluence-switching-w-to-s");
        }
    }

    #[test]
    fn a_warning_points_at_the_rows_the_floats_will_reach() {
        let world = confluence();
        let slots = arm_slots(&world);
        let mut gate = world.gate.clone();
        gate.tide = Tide::Open;
        gate.dry_arm = Arm(1);
        gate.warning_until = Some(500);
        gate.switch_target = Some(Arm(0));
        let motion = Motion::rest(rest_pose(&gate, &slots), 0);
        let shown = drawn(&motion, &gate, &slots, 400, Some(1));
        // W fills and E drains; S stays deep.
        assert_eq!(
            shown.pointers,
            vec![(Slot::Left, 20, 1), (Slot::Right, 0, 1)]
        );
        assert_eq!(shown.wheel, (Slot::Left.wheel() + 1) % 8);
        let keys = pieces(&shown);
        assert_eq!(keys[0], (BASE_KEY.to_string(), 0, 0));
        assert!(keys.contains(&("station3_pointer_1".to_string(), 46, 20)));
        assert!(keys.contains(&("station3_float_side_1".to_string(), 0, 0)));
        assert!(keys.contains(&("station3_float_side_21".to_string(), 46, 0)));
        assert!(keys.contains(&("station3_float_front_17".to_string(), 0, 0)));
    }
}
