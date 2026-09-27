//! Trial 12's field readability and economy nudges (fix stream W4).
//!
//! The blind three-way match on the Confluence found a field that hid what
//! mattered: own Loom rings drawn red like the enemy's, deployed guns sat
//! out of reach with no sign, pressure capped for twenty minutes, home
//! wrecks and Works queues ran dry in silence, wells and tidal ground were
//! invisible while placing, and units vanished behind the station's rails.
//!
//! Everything here is presentation: it reads the world this seat sees and
//! never changes it. Marks are drawn in code (no new art); words stay under
//! the pointer and on three short alert cards.

use crate::canvas::{
    Atlas, Canvas, Color, GOLD, INK, JADE, PANEL, RED, SHADOW_CAST, SHADOW_CONTACT,
};
use crate::field_alerts::{AlertHistory, AlertKind, FieldAlert};
use crate::game::{Action, Game, Mode};
use crate::occlusion::PixelScale;
use bw_content::{LOOM_MIN_RANGE, spec};
use bw_core::{Camera, EntityId, FP, Kind, Pos};
use bw_sim::{Order, World};
use std::collections::BTreeMap;

/// Pressure stands at its cap this long before the card is raised.
pub const PRESSURE_GRACE: u64 = 5 * 30;
/// While pressure stays full the card comes back this often.
pub const PRESSURE_REPEAT: u64 = 90 * 30;
/// A wreck this close to the HQ or a salvage yard is a home wreck.
pub const HOME_WRECK_CELLS: i32 = 14;
/// A Works with an empty queue this long is idle.
pub const PRODUCER_IDLE: u64 = 20 * 30;
/// While producers stay idle and a machine is affordable, the card comes
/// back this often.
pub const PRODUCER_REPEAT: u64 = 180 * 30;
/// A deployed gun with no enemy in reach this long, while one is near,
/// is marked.
pub const STRANDED_GRACE: u64 = 3 * 30;
/// How far beyond its reach an enemy counts as near a deployed gun.
pub const NEAR_CELLS: i32 = 4;
/// Own reach rings are drawn for this many selected guns at most; beyond
/// it only the one under the pointer shows its ring (trial 12: eight
/// overlapping rings at the station).
pub const RING_SELECTION_LIMIT: usize = 3;
/// Faint enemy reach rings: at most this many.
pub const ENEMY_RING_LIMIT: usize = 6;
/// How strongly a faint enemy ring is drawn: quiet, but it must hold on
/// sand.
pub const ENEMY_RING_ALPHA: u8 = 170;
/// A wreck heap's salvage pips hide while a fight is this close.
pub const FIGHT_CELLS: i32 = 6;

/// The hatch over ground that refuses a building: a quiet red.
pub const REFUSED_HATCH: Color = [226, 108, 84, 120];

/// A filled triangle whose tip is (x, y), pointing along (dx, dy).
pub fn edge_arrow(canvas: &mut Canvas, x: i32, y: i32, dx: i32, dy: i32, color: Color) {
    let len = f64::from(dx).hypot(f64::from(dy)).max(1.0);
    let (ux, uy) = (f64::from(dx) / len, f64::from(dy) / len);
    let (length, half) = (12.0, 6.0);
    for py in y - 13..=y + 13 {
        for px in x - 13..=x + 13 {
            let (rx, ry) = (f64::from(px - x), f64::from(py - y));
            let back = -(rx * ux + ry * uy);
            let across = (rx * uy - ry * ux).abs();
            if (0.0..=length).contains(&back) && across <= half * back / length + 0.5 {
                let edge = across > half * back / length - 1.0 || back > length - 1.0;
                canvas.pixel(px, py, if edge { INK } else { color });
            }
        }
    }
}

/// The colour of this seat's own reach: gold, the colour of everything
/// that is yours on the field, never an enemy's red.
pub const OWN_RING: Color = GOLD;

/// One reach ring to draw: the gun's reach (broken), its blind zone
/// (whole, 0 for none), and whether the gun is ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReachRing {
    pub pos: Pos,
    pub reach: i32,
    pub blind: i32,
    pub color: Color,
    pub own: bool,
}

/// Presentation memory for the nudges; none of it is saved.
#[derive(Default, Debug)]
pub struct FieldNudges {
    last_tick: u64,
    /// Each own deployed gun: the last tick it had an enemy in reach (or
    /// the tick it settled).
    in_reach: BTreeMap<EntityId, u64>,
    full_since: Option<u64>,
    full_card: Option<u64>,
    /// Known live home wrecks at the last look.
    home_live: Option<usize>,
    /// Each own Works: since when its queue has stood empty.
    idle_since: BTreeMap<EntityId, u64>,
    idle_card: Option<u64>,
}

/// Whether a kind is a gun whose reach the field rings.
pub fn ringed_gun(kind: Kind) -> bool {
    matches!(kind, Kind::Loom | Kind::Heliostat | Kind::Tower)
}

/// Whether a kind is a deployed gun that can be stranded out of reach.
fn deployable_gun(kind: Kind) -> bool {
    matches!(kind, Kind::Loom | Kind::Heliostat)
}

fn settled(e: &bw_sim::Entity) -> bool {
    e.hp > 0 && e.aboard.is_none() && e.deployed && e.deploy_remaining == 0
}

fn min_reach(kind: Kind) -> i32 {
    if kind == Kind::Loom {
        LOOM_MIN_RANGE
    } else {
        0
    }
}

/// The visible enemies of seat 0.
fn visible_enemies(world: &World) -> impl Iterator<Item = &bw_sim::Entity> {
    world.entities.iter().filter(move |t| {
        t.owner != 0 && t.hp > 0 && t.aboard.is_none() && world.entity_visible(0, t.id)
    })
}

fn in_reach(world: &World, gun: &bw_sim::Entity, target: &bw_sim::Entity) -> bool {
    let d = gun.pos.distance_sq(target.pos);
    d <= i64::from(world.weapon_range(gun)).pow(2) && d >= i64::from(min_reach(gun.kind)).pow(2)
}

/// A deployed gun of ours that cannot reach what it should hit: its
/// order's target, or the nearest enemy near it after a spell with none
/// in reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReachMiss {
    pub gun: EntityId,
    pub from: Pos,
    pub target: Pos,
    pub reach: i32,
}

impl ReachMiss {
    /// Whole cells the target stands beyond the gun's reach (at least 1).
    pub fn short_cells(&self) -> i32 {
        let d = ((self.from.distance_sq(self.target)) as f64).sqrt();
        ((d - f64::from(self.reach)) / f64::from(FP))
            .ceil()
            .max(1.0) as i32
    }
}

/// Every own deployed gun out of reach of its target now.
pub fn reach_misses(world: &World, nudges: &FieldNudges) -> Vec<ReachMiss> {
    let mut out = Vec::new();
    for gun in world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && deployable_gun(e.kind) && settled(e))
    {
        let reach = world.weapon_range(gun);
        // An order's own target, seen and out of reach: at once.
        if let Order::Attack { target } = gun.order
            && let Some(t) = visible_enemies(world).find(|t| t.id == target)
        {
            if !in_reach(world, gun, t) {
                out.push(ReachMiss {
                    gun: gun.id,
                    from: gun.pos,
                    target: t.pos,
                    reach,
                });
            }
            continue;
        }
        // No enemy in reach for a spell while one stands near.
        let quiet = nudges
            .in_reach
            .get(&gun.id)
            .is_some_and(|at| world.tick.saturating_sub(*at) >= STRANDED_GRACE);
        if !quiet || visible_enemies(world).any(|t| in_reach(world, gun, t)) {
            continue;
        }
        let near = i64::from(reach + NEAR_CELLS * FP).pow(2);
        if let Some(t) = visible_enemies(world)
            .filter(|t| {
                let d = gun.pos.distance_sq(t.pos);
                d <= near && d > i64::from(reach).pow(2)
            })
            .min_by_key(|t| (gun.pos.distance_sq(t.pos), t.id))
        {
            out.push(ReachMiss {
                gun: gun.id,
                from: gun.pos,
                target: t.pos,
                reach,
            });
        }
    }
    out
}

/// The own HQ, if it stands.
fn own_hq(world: &World) -> Option<&bw_sim::Entity> {
    world
        .entities
        .iter()
        .find(|e| e.owner == 0 && e.kind == Kind::Headquarters && e.hp > 0)
}

/// Whether a wreck is live as this seat knows it: the salvage it holds
/// when seen, else what was seen of it last (a wreck never seen is live).
fn known_live(world: &World, stages: &BTreeMap<u32, u8>, r: &bw_sim::Resource) -> bool {
    if world.visible(0, r.pos) {
        r.remaining > 0
    } else {
        stages.get(&r.id) != Some(&3)
    }
}

/// The known live wrecks the workers take on their own near home: off the
/// lanes and the sluice island, within `HOME_WRECK_CELLS` of the HQ or a
/// salvage yard.
pub fn home_live_wrecks(world: &World, stages: &BTreeMap<u32, u8>) -> usize {
    let homes: Vec<Pos> = world
        .entities
        .iter()
        .filter(|e| {
            e.owner == 0
                && e.hp > 0
                && e.build_remaining == 0
                && matches!(e.kind, Kind::Headquarters | Kind::Dropoff)
        })
        .map(|e| e.pos)
        .collect();
    let limit = i64::from(HOME_WRECK_CELLS * FP).pow(2);
    world
        .map
        .resources
        .iter()
        .filter(|r| {
            let (x, y) = r.pos.cell_xy();
            !world.map.terrain(x, y).is_tidal()
                && !world.map.island_cell(x, y)
                && homes.iter().any(|h| h.distance_sq(r.pos) <= limit)
                && known_live(world, stages, r)
        })
        .count()
}

/// Works of ours standing with an empty queue, not researching.
fn idle_works(world: &World) -> Vec<&bw_sim::Entity> {
    let research = world.players[0].research.as_ref().map(|r| r.building);
    world
        .entities
        .iter()
        .filter(|e| {
            e.owner == 0
                && e.hp > 0
                && e.kind == Kind::Works
                && e.build_remaining == 0
                && e.queue.is_empty()
                && research != Some(e.id)
        })
        .collect()
}

/// Whether a Works could start a machine now: salvage, pressure and crew.
pub fn machine_affordable(world: &World) -> bool {
    let p = &world.players[0];
    p.faction.army().iter().any(|&k| {
        let s = spec(k);
        s.salvage <= p.salvage && s.pressure <= p.pressure && p.crew + s.crew <= p.cap
    })
}

impl FieldNudges {
    /// Follow the field after an authoritative tick and raise the nudge
    /// cards.
    pub fn observe(
        &mut self,
        world: &World,
        alerts: &mut AlertHistory,
        stages: &BTreeMap<u32, u8>,
    ) {
        let tick = world.tick;
        if tick < self.last_tick {
            *self = FieldNudges::default();
        }
        self.last_tick = tick;
        if world.players.is_empty() || world.is_eliminated(0) {
            return;
        }
        self.track_guns(world);
        self.track_pressure(world, alerts);
        if tick.is_multiple_of(30) {
            self.track_wrecks(world, alerts, stages);
        }
        self.track_producers(world, alerts);
    }

    fn track_guns(&mut self, world: &World) {
        let guns: Vec<&bw_sim::Entity> = world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && deployable_gun(e.kind) && settled(e))
            .collect();
        self.in_reach
            .retain(|id, _| guns.iter().any(|g| g.id == *id));
        for gun in guns {
            let hit = visible_enemies(world).any(|t| in_reach(world, gun, t));
            let entry = self.in_reach.entry(gun.id).or_insert(world.tick);
            if hit {
                *entry = world.tick;
            }
        }
    }

    fn track_pressure(&mut self, world: &World, alerts: &mut AlertHistory) {
        let p = &world.players[0];
        let full = p.pressure_cap > 0 && p.pressure >= p.pressure_cap;
        if !full {
            self.full_since = None;
            self.full_card = None;
            alerts.entries.retain(|a| a.kind != AlertKind::PressureFull);
            return;
        }
        let since = *self.full_since.get_or_insert(world.tick);
        let due = self
            .full_card
            .is_none_or(|at| world.tick.saturating_sub(at) >= PRESSURE_REPEAT);
        if world.tick.saturating_sub(since) >= PRESSURE_GRACE && due {
            let at = own_hq(world).map_or_else(|| world.start_of(0), |e| e.pos);
            alerts.push(AlertKind::PressureFull, at, world.tick);
            self.full_card = Some(world.tick);
        }
    }

    fn track_wrecks(
        &mut self,
        world: &World,
        alerts: &mut AlertHistory,
        stages: &BTreeMap<u32, u8>,
    ) {
        let Some(hq) = own_hq(world) else {
            return;
        };
        let live = home_live_wrecks(world, stages);
        if live > 0 {
            alerts.entries.retain(|a| a.kind != AlertKind::WrecksOut);
        }
        let workers = world
            .entities
            .iter()
            .any(|e| e.owner == 0 && e.hp > 0 && e.kind.is_worker());
        if self.home_live.is_some_and(|before| before > 0) && live == 0 && workers {
            // The card stands at the wreck to go to, so its echo on the
            // chart pings where the salvage is; the base when none is known.
            let at = nearest_reachable_wreck(world, stages).unwrap_or(hq.pos);
            alerts.push(AlertKind::WrecksOut, at, world.tick);
        }
        self.home_live = Some(live);
    }

    fn track_producers(&mut self, world: &World, alerts: &mut AlertHistory) {
        let idle = idle_works(world);
        self.idle_since
            .retain(|id, _| idle.iter().any(|e| e.id == *id));
        for e in &idle {
            self.idle_since.entry(e.id).or_insert(world.tick);
        }
        let long: Vec<(EntityId, Pos)> = idle
            .iter()
            .filter(|e| {
                self.idle_since
                    .get(&e.id)
                    .is_some_and(|at| world.tick.saturating_sub(*at) >= PRODUCER_IDLE)
            })
            .map(|e| (e.id, e.pos))
            .collect();
        let affordable = machine_affordable(world);
        if long.is_empty() || !affordable {
            alerts
                .entries
                .retain(|a| a.kind != AlertKind::ProducersIdle);
            if long.is_empty() {
                self.idle_card = None;
            }
            return;
        }
        if let Some(card) = alerts
            .entries
            .iter_mut()
            .find(|a| a.kind == AlertKind::ProducersIdle)
        {
            card.count = long.len();
        }
        let due = self
            .idle_card
            .is_none_or(|at| world.tick.saturating_sub(at) >= PRODUCER_REPEAT);
        if due {
            alerts
                .entries
                .retain(|a| a.kind != AlertKind::ProducersIdle);
            alerts.push(AlertKind::ProducersIdle, long[0].1, world.tick);
            if let Some(card) = alerts.entries.first_mut() {
                card.count = long.len();
                let name = crate::ux::building_name(Kind::Works, world.players[0].faction);
                card.subject = (name != Kind::Works.name()).then_some(name);
            }
            self.idle_card = Some(world.tick);
        }
    }
}

/// The words of a nudge card: short, the picture beside them says the rest.
pub fn nudge_label(card: &FieldAlert) -> String {
    match card.kind {
        AlertKind::PressureFull => "PRESSURE FULL".into(),
        AlertKind::WrecksOut => "WRECKS OUT".into(),
        AlertKind::ProducersIdle => {
            let name = card.subject.unwrap_or("WORKS");
            if card.count > 1 {
                format!("{} {name} IDLE", card.count)
            } else {
                format!("{name} IDLE")
            }
        }
        _ => String::new(),
    }
}

/// Whether an alert kind is one of the nudges drawn with a picture.
pub fn is_nudge(kind: AlertKind) -> bool {
    matches!(
        kind,
        AlertKind::PressureFull | AlertKind::WrecksOut | AlertKind::ProducersIdle
    )
}

/// The picture on a nudge card: RECLAIM's for pressure, GATHER's for the
/// wrecks, the side's Works for idle production.
pub fn nudge_icon(kind: AlertKind, faction: bw_core::Faction) -> Option<String> {
    Some(match kind {
        AlertKind::PressureFull => "ui_cmd_reclaim".into(),
        AlertKind::WrecksOut => "ui_cmd_gather".into(),
        AlertKind::ProducersIdle => format!("ui_build_{}", Kind::Works.asset(faction)),
        _ => return None,
    })
}

/// Free wells: no condenser of ours, nor one of theirs in sight, on them.
pub fn free_wells(world: &World) -> Vec<Pos> {
    world
        .map
        .wells
        .iter()
        .copied()
        .filter(|w| {
            !world.entities.iter().any(|e| {
                e.hp > 0
                    && e.kind == Kind::Condenser
                    && (e.owner == 0 || world.entity_visible(0, e.id))
                    && e.pos.distance_sq(*w) <= i64::from(FP).pow(2)
            })
        })
        .collect()
}

/// Whether no building may stand on a cell for a reason the ground itself
/// hides: a lane that is dry or shallow now (tidal ground), a causeway, or
/// the sluice's approach. Deep water and rock need no mark.
pub fn refused_ground(world: &World, x: i32, y: i32) -> bool {
    let t = world.map.terrain(x, y);
    if t.is_tidal() {
        return !matches!(world.depth_at(x, y), Some(bw_sim::Depth::Deep));
    }
    matches!(t, bw_core::Terrain::Salt | bw_core::Terrain::Silt)
        && Pos::cell(x, y).distance_sq(world.map.gate_pos) <= i64::from(FP * 4).pow(2)
}

/// Hatch every cell in view where no building may stand for a reason the
/// ground hides (see `refused_ground`).  Only ground the side has seen:
/// hatching the unknown would read as noise and tell more than the chart
/// knows.
pub(crate) fn hatch_refused_ground(
    canvas: &mut Canvas,
    world: &World,
    explored: Option<&crate::chart_memory::Explored>,
    camera: Camera,
) {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    for y in 0..i32::from(world.map.height) {
        for x in 0..i32::from(world.map.width) {
            let (px, py) = camera.project(Pos::cell(x, y));
            if px < -16 || py < -8 || px > w + 16 || py > h + 8 {
                continue;
            }
            let seen = world.visible(0, Pos::cell(x, y)) || explored.is_none_or(|e| e.get(x, y));
            if seen && refused_ground(world, x, y) {
                hatch_cell(canvas, px, py, REFUSED_HATCH);
            }
        }
    }
}

/// The nearest wreck to the base the side knows holds salvage and its
/// workers can walk to.
pub fn nearest_reachable_wreck(world: &World, stages: &BTreeMap<u32, u8>) -> Option<Pos> {
    let home = own_hq(world).map_or_else(|| world.start_of(0), |e| e.pos);
    let worker: Vec<EntityId> = world
        .entities
        .iter()
        .filter(|e| e.owner == 0 && e.hp > 0 && e.kind.is_worker() && e.aboard.is_none())
        .map(|e| e.id)
        .take(1)
        .collect();
    let mut live: Vec<&bw_sim::Resource> = world
        .map
        .resources
        .iter()
        .filter(|r| known_live(world, stages, r) && world.gatherable(Kind::Headquarters, r))
        .collect();
    live.sort_by_key(|r| (r.pos.distance_sq(home), r.id));
    live.into_iter()
        .take(12)
        .find(|r| worker.is_empty() || world.any_can_reach(&worker, r.pos))
        .map(|r| r.pos)
}

/// A light diagonal hatch over one cell's diamond: ground that refuses a
/// building, told apart from the jade of ground that takes one.
pub fn hatch_cell(canvas: &mut Canvas, cx: i32, cy: i32, color: Color) {
    for dy in -7i32..=7 {
        let half = (8 - dy.abs()) * 2 - 1;
        for dx in -half..=half {
            if (dx + 2 * dy).rem_euclid(6) == 0 {
                canvas.pixel(cx + dx, cy + dy, color);
            }
        }
    }
}

/// A ring on the ground in the world's projection, broken into dashes or
/// whole.
pub fn ground_ring(
    canvas: &mut Canvas,
    camera: Camera,
    pos: Pos,
    radius: i32,
    color: Color,
    broken: bool,
) {
    const STEPS: usize = 32;
    let points: Vec<(i32, i32)> = (0..STEPS)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / STEPS as f64;
            camera.project(Pos {
                x: pos.x + (f64::from(radius) * a.cos()).round() as i32,
                y: pos.y + (f64::from(radius) * a.sin()).round() as i32,
            })
        })
        .collect();
    for i in 0..STEPS {
        if broken && i % 2 == 1 {
            continue;
        }
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % STEPS];
        canvas.line(x0, y0, x1, y1, color);
    }
}

fn with_alpha(c: Color, a: u8) -> Color {
    [c[0], c[1], c[2], a]
}

/// A line of short dashes from `a` to `b`.
fn dashes(canvas: &mut Canvas, a: (i32, i32), b: (i32, i32), on: i32, off: i32, color: Color) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs()).max(1);
    let mut i = 0;
    while i < steps {
        let end = (i + on).min(steps);
        canvas.line(
            a.0 + dx * i / steps,
            a.1 + dy * i / steps,
            a.0 + dx * end / steps,
            a.1 + dy * end / steps,
            color,
        );
        i += on + off;
    }
}

/// Draw a spent wreck: its art pressed flat to the ground and drained of
/// colour, so it never reads as salvage (trial 12: emptied debris looked
/// live and took a Move order). Baked shadow texels are left out.
pub fn draw_spent(
    atlas: &Atlas,
    canvas: &mut Canvas,
    key: &str,
    x: i32,
    y: i32,
    dim: bool,
) -> bool {
    let Some(s) = atlas.sprites.get(key) else {
        return false;
    };
    let left = x - s.anchor_x;
    // Rows above the anchor are pressed to half their height.
    let above = s.anchor_y.max(0);
    let rows = above / 2 + (s.h as i32 - above).max(0);
    for row in 0..rows {
        let (src, py) = if row < above / 2 {
            (above - (above / 2 - row) * 2, y - (above / 2 - row))
        } else {
            let below = row - above / 2;
            (above + below, y + below)
        };
        if src < 0 || src >= s.h as i32 {
            continue;
        }
        for xx in 0..s.w {
            let Some(c) = atlas.sample(s, xx, src as u32).filter(|c| c[3] != 0) else {
                continue;
            };
            if c == SHADOW_CAST || c == SHADOW_CONTACT {
                continue;
            }
            let c = spent_colour(c);
            let c = if dim {
                crate::chart_surface::chart_color(c)
            } else {
                c
            };
            canvas.pixel(left + xx as i32, py, c);
        }
    }
    true
}

/// A texel of spent scrap: grey, low in contrast, a little dark.
pub fn spent_colour(c: Color) -> Color {
    let luma = (u32::from(c[0]) * 3 + u32::from(c[1]) * 6 + u32::from(c[2])) / 10;
    // Pull toward a pale ash: a spent heap is dust on the salt, never a
    // dark patch that could read as a shadow or a hole.
    let v = (luma * 4 + 170 * 6) / 10;
    let v = v.min(255);
    [v as u8, (v * 98 / 100) as u8, (v * 92 / 100) as u8, c[3]]
}

/// Whether a fight is on near a wreck heap: a shot landed close within the
/// last seconds, or an armed enemy stands close.
pub fn fight_near(world: &World, shots: &[Pos], at: Pos) -> bool {
    let reach = i64::from(FIGHT_CELLS * FP).pow(2);
    shots.iter().any(|p| p.distance_sq(at) <= reach)
        || visible_enemies(world).any(|e| {
            !e.kind.is_worker()
                && !e.kind.is_building()
                && spec(e.kind).damage > 0
                && e.pos.distance_sq(at) <= reach
        })
}

/// A drawn machine that may stand behind the station, for its ghost.
pub struct GhostUnit {
    pub key: String,
    pub x: i32,
    pub y: i32,
    pub scale: PixelScale,
    pub tint: Color,
}

/// Machines behind the station's railings, wheels and shafts keep a tinted
/// ghost through the station's pixels, as they do behind a building
/// (trial 12: units piled up unseen behind the station's railings).
pub fn ghost_behind_station(
    atlas: &Atlas,
    canvas: &mut Canvas,
    units: &[GhostUnit],
    parts: &[(String, i32, i32)],
) -> usize {
    let rect = |key: &str, x: i32, y: i32, scale: PixelScale| {
        atlas.sprites.get(key).map(|s| {
            let left = x - scale.apply(s.anchor_x);
            let top = y - scale.apply(s.anchor_y);
            (
                left,
                top,
                left + scale.apply(s.w as i32),
                top + scale.apply(s.h as i32),
            )
        })
    };
    let mut drawn = 0;
    for unit in units {
        let Some((al, at, ar, ab)) = rect(&unit.key, unit.x, unit.y, unit.scale) else {
            continue;
        };
        for (key, px, py) in parts {
            let Some((bl, bt, br, bb)) = rect(key, *px, *py, PixelScale::ONE) else {
                continue;
            };
            if !(al < br && bl < ar && at < bb && bt < ab) {
                continue;
            }
            if atlas.draw_ghost_through(
                canvas,
                &unit.key,
                unit.x,
                unit.y,
                unit.scale,
                key,
                *px,
                *py,
                PixelScale::ONE,
                unit.tint,
            ) {
                drawn += 1;
            }
        }
    }
    drawn
}

impl Game {
    /// A nudge card's click: the HQ for pressure, the nearest live wreck
    /// the workers can reach, the idle producers. False for other cards.
    pub(crate) fn focus_nudge(&mut self, kind: AlertKind) -> bool {
        match kind {
            AlertKind::PressureFull => {
                let Some((id, pos)) = own_hq(&self.world).map(|e| (e.id, e.pos)) else {
                    return false;
                };
                self.selected = vec![id];
                self.look_at(pos);
                true
            }
            AlertKind::WrecksOut => {
                if let Some(pos) = self.nearest_reachable_wreck() {
                    self.look_at(pos);
                    true
                } else {
                    false
                }
            }
            AlertKind::ProducersIdle => {
                let idle: Vec<(EntityId, Pos)> = idle_works(&self.world)
                    .iter()
                    .map(|e| (e.id, e.pos))
                    .collect();
                let Some(&(_, first)) = idle.first() else {
                    return false;
                };
                self.selected = idle.iter().map(|(id, _)| *id).collect();
                self.look_at(first);
                true
            }
            _ => false,
        }
    }

    fn look_at(&mut self, pos: Pos) {
        self.camera.center(pos);
        self.camera_sub = (0.0, 0.0);
        self.drag = None;
        self.ux.minimap_drag = false;
        self.mode = Mode::Context;
        self.last_click = None;
    }

    /// The live wreck nearest the HQ that a worker of ours can walk to.
    pub(crate) fn nearest_reachable_wreck(&self) -> Option<Pos> {
        nearest_reachable_wreck(&self.world, &self.resource_stages)
    }

    /// Reach rings (trial 12): own guns in gold, only for the selected
    /// (three at most) and the one under the pointer; an enemy gun's reach
    /// in its colour under the pointer, and faintly while it covers a
    /// machine of ours that is selected.
    pub(crate) fn reach_ring_plan(&self) -> Vec<ReachRing> {
        let hovered = self.hovered;
        let own: Vec<EntityId> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.aboard.is_none()
                    && e.build_remaining == 0
                    && ringed_gun(e.kind)
                    && self.selected.contains(&e.id)
            })
            .map(|e| e.id)
            .collect();
        let few = own.len() <= RING_SELECTION_LIMIT;
        let mut rings = Vec::new();
        for e in self.world.entities.iter().filter(|e| {
            e.owner == 0
                && e.hp > 0
                && e.aboard.is_none()
                && e.build_remaining == 0
                && ringed_gun(e.kind)
        }) {
            let over = hovered == Some(e.id);
            if !(over || (few && own.contains(&e.id))) {
                continue;
            }
            let lone = own.len() <= 1 || over;
            rings.push(ReachRing {
                pos: e.pos,
                reach: self.world.weapon_range(e),
                blind: if lone { min_reach(e.kind) } else { 0 },
                color: OWN_RING,
                own: true,
            });
        }
        // The enemy's reach: the gun under the pointer, and faintly the
        // guns whose reach covers a selected machine of ours.
        let chosen: Vec<Pos> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0 && e.hp > 0 && !e.kind.is_building() && self.selected.contains(&e.id)
            })
            .map(|e| e.pos)
            .collect();
        let mut enemy: Vec<(i64, &bw_sim::Entity, bool)> = visible_enemies(&self.world)
            .filter(|e| {
                ringed_gun(e.kind)
                    && e.build_remaining == 0
                    && (e.kind == Kind::Tower || settled(e))
            })
            .filter_map(|e| {
                let over = hovered == Some(e.id);
                let reach = i64::from(self.world.weapon_range(e) + 2 * FP).pow(2);
                let near = chosen
                    .iter()
                    .map(|p| p.distance_sq(e.pos))
                    .filter(|d| *d <= reach)
                    .min();
                (over || near.is_some()).then(|| (near.unwrap_or(0), e, over))
            })
            .collect();
        enemy.sort_by_key(|(d, e, over)| (!*over, *d, e.id));
        for (_, e, over) in enemy.into_iter().take(ENEMY_RING_LIMIT) {
            let hue = crate::seats::seat_colour(&self.world, e.owner);
            rings.push(ReachRing {
                pos: e.pos,
                reach: self.world.weapon_range(e),
                blind: 0,
                color: if over {
                    hue
                } else {
                    with_alpha(hue, ENEMY_RING_ALPHA)
                },
                own: false,
            });
        }
        rings
    }

    pub(crate) fn draw_reach_rings(&mut self) {
        let camera = self.camera;
        for ring in self.reach_ring_plan() {
            ground_ring(
                &mut self.canvas,
                camera,
                ring.pos,
                ring.reach,
                ring.color,
                true,
            );
            if ring.blind > 0 {
                // A Loom's blind zone: a thin whole ring, quieter.
                ground_ring(
                    &mut self.canvas,
                    camera,
                    ring.pos,
                    ring.blind,
                    with_alpha(ring.color, 150),
                    false,
                );
            }
        }
    }

    /// A deployed gun of ours that cannot reach: gold dashes out to where
    /// its reach ends, a bar across there, and dots on to the target
    /// (trial 12: Looms sat a cell out of a nest's reach while it killed
    /// three). The words are under the pointer at the bar.
    pub(crate) fn draw_reach_misses(&mut self) {
        let camera = self.camera;
        let misses = reach_misses(&self.world, &self.ux_nudges);
        let lift = 6;
        for (i, miss) in misses.iter().enumerate() {
            let (gx, gy) = camera.project(miss.from);
            let (tx, ty) = camera.project(miss.target);
            let (dx, dy) = ((tx - gx) as f64, (ty - gy) as f64);
            let len = dx.hypot(dy).max(1.0);
            // One line to each target, from the gun nearest it; the other
            // guns short of the same target point at it with a stub.
            let lead = !misses.iter().enumerate().any(|(j, other)| {
                other.target == miss.target
                    && (other.from.distance_sq(other.target), j)
                        < (miss.from.distance_sq(miss.target), i)
            });
            if !lead {
                let (sx, sy) = ((dx / len * 14.0) as i32, (dy / len * 14.0) as i32);
                dashes(
                    &mut self.canvas,
                    (gx, gy - lift),
                    (gx + sx, gy - lift + sy),
                    4,
                    3,
                    OWN_RING,
                );
                continue;
            }
            let d = (miss.from.distance_sq(miss.target) as f64).sqrt().max(1.0);
            let f = (f64::from(miss.reach) / d).min(1.0);
            let end = Pos {
                x: miss.from.x + ((miss.target.x - miss.from.x) as f64 * f) as i32,
                y: miss.from.y + ((miss.target.y - miss.from.y) as f64 * f) as i32,
            };
            let (ex, ey) = camera.project(end);
            dashes(
                &mut self.canvas,
                (gx, gy - lift),
                (ex, ey - lift),
                4,
                3,
                OWN_RING,
            );
            // The bar across the line where the reach ends.
            let (px, py) = ((-dy / len * 5.0) as i32, (dx / len * 5.0) as i32);
            for o in 0..2 {
                self.canvas.line(
                    ex - px,
                    ey - lift - py + o,
                    ex + px,
                    ey - lift + py + o,
                    OWN_RING,
                );
            }
            // Dots across the gap to the target, and a hollow mark on it.
            dashes(
                &mut self.canvas,
                (ex, ey - lift),
                (tx, ty - lift),
                1,
                4,
                with_alpha(RED, 200),
            );
            for (ax, ay, bx, by) in [(-4, 0, 0, -2), (0, -2, 4, 0), (4, 0, 0, 2), (0, 2, -4, 0)] {
                self.canvas
                    .line(tx + ax, ty - lift + ay, tx + bx, ty - lift + by, RED);
            }
            let short = miss.short_cells();
            self.field_tips.push(
                crate::field_labels::FieldTip::new(
                    crate::field_labels::Area::around((ex, ey - lift), 8, 8, 8, 8),
                    (ex, ey - lift - 8),
                    "OUT OF REACH",
                    GOLD,
                )
                .line(format!(
                    "{short} cell{} short. Pack and close in.",
                    if short == 1 { "" } else { "s" }
                )),
            );
        }
    }

    /// While a condenser is placed: each free well in view gets its
    /// footprint filled and a bobbing arrow over its pump; with none in view, an
    /// arrow on the edge points to the nearest.
    pub(crate) fn point_to_wells(&mut self, camera: Camera, wells: &[Pos], tick: u64) {
        let (w, h) = (self.canvas.width() as i32, self.canvas.height() as i32);
        let on = |(x, y): (i32, i32)| x >= 0 && y >= 0 && x < w && y < h;
        let lit = (tick / 15).is_multiple_of(2);
        let fill = with_alpha(JADE, if lit { 120 } else { 70 });
        let mut any = false;
        for well in wells {
            let centre = camera.project(*well);
            if !on(centre) {
                continue;
            }
            any = true;
            let (wx, wy) = well.cell_xy();
            let n = spec(Kind::Condenser).footprint;
            for a in 0..n {
                for b in 0..n {
                    let (qx, qy) = camera.project(Pos::cell(wx + a, wy + b));
                    self.canvas.diamond(qx, qy, 15, 7, fill);
                }
            }
            // An arrow over the pump's top, bobbing, reads from across the
            // base and over the workers crowding a well.
            let pole = self
                .atlas
                .as_ref()
                .and_then(|a| a.sprites.get("well"))
                .map_or(44, |s| s.anchor_y - s.opaque_top as i32);
            let bob = [0, 1, 2, 1][((tick / 8) % 4) as usize];
            edge_arrow(
                &mut self.canvas,
                centre.0,
                centre.1 - pole - 14 + bob,
                0,
                1,
                JADE,
            );
        }
        if any {
            return;
        }
        let middle = (w / 2, h / 2);
        let Some(nearest) = wells.iter().min_by_key(|p| {
            let (x, y) = camera.project(**p);
            i64::from(x - middle.0).pow(2) + i64::from(y - middle.1).pow(2)
        }) else {
            return;
        };
        let target = camera.project(*nearest);
        if let Some((ex, ey)) = crate::tactics::screen_edge_point(middle, target, w, h) {
            edge_arrow(
                &mut self.canvas,
                ex,
                ey,
                target.0 - middle.0,
                target.1 - middle.1,
                JADE,
            );
        }
    }

    /// The nudges on the interface: a picture on each nudge card, RECLAIM
    /// lit while pressure is full, the free wells on the chart while a
    /// condenser is placed.
    pub(crate) fn draw_nudge_hud(&mut self) {
        let s = self.ui_scale();
        let faction = self.world.players[0].faction;
        let blink = (self.world.tick / 15).is_multiple_of(2);
        let cards: Vec<(crate::game::Button, AlertKind, String)> = self
            .buttons
            .iter()
            .filter_map(|b| match b.action {
                Action::FocusAlert(Some(id)) => self
                    .ux
                    .alerts
                    .entries
                    .iter()
                    .find(|a| a.id == id && is_nudge(a.kind))
                    .map(|a| (b.clone(), a.kind, a.label())),
                _ => None,
            })
            .collect();
        for (b, kind, label) in cards {
            let hover = b.contains(self.cursor.0, self.cursor.1);
            // The card redrawn with its picture first and its words after.
            self.canvas.rect(
                b.x,
                b.y,
                b.w,
                b.h,
                if hover { crate::canvas::EDGE } else { PANEL },
            );
            self.canvas.rect(b.x, b.y, 2 * s, b.h, JADE);
            let icon = 18 * s;
            let drawn = nudge_icon(kind, faction).is_some_and(|key| {
                self.atlas.as_ref().is_some_and(|atlas| {
                    atlas.draw_scaled(
                        &mut self.canvas,
                        &key,
                        b.x + 4 * s,
                        b.y + (b.h - icon) / 2,
                        false,
                        PixelScale {
                            numerator: 3 * s as u16,
                            denominator: 4,
                        },
                    )
                })
            });
            let x = b.x + if drawn { 4 * s + icon + 3 * s } else { 6 * s };
            crate::native_ui::small(
                &mut self.canvas,
                &crate::native_ui::fit_small(&label, b.x + b.w - x - 2 * s, s),
                x,
                b.y + (b.h - 7 * s) / 2,
                crate::canvas::WHITE,
                s,
            );
            if kind == AlertKind::PressureFull && blink {
                self.canvas.frame(b.x, b.y, b.w, b.h, GOLD);
            }
        }
        // RECLAIM lights up on the HQ's card while pressure is full.
        let p = &self.world.players[0];
        if p.pressure_cap > 0 && p.pressure >= p.pressure_cap {
            let reclaim: Vec<crate::game::Button> = self
                .buttons
                .iter()
                .filter(|b| b.action == Action::Reclaim && b.enabled)
                .cloned()
                .collect();
            for b in reclaim {
                let c = if blink { GOLD } else { crate::canvas::WHITE };
                for i in 0..s {
                    self.canvas.frame(
                        b.x - s + i,
                        b.y - s + i,
                        b.w + 2 * s - 2 * i,
                        b.h + 2 * s - 2 * i,
                        c,
                    );
                }
            }
        }
        // Wreck beds seen spent fade on the chart: the mark stays (the bed
        // is geography) but in a pale ink, so the live ones stand out.
        {
            let chart = self.chart();
            let faded = crate::minimap_chart::PAPER_AGED;
            let spent: Vec<Pos> = self
                .world
                .map
                .resources
                .iter()
                .filter(|r| self.resource_stages.get(&r.id) == Some(&3))
                .filter(|r| !r.scrap || self.world.visible(0, r.pos))
                .map(|r| r.pos)
                .collect();
            for pos in spent {
                let (x, y) = chart.plot(&self.world, pos);
                for d in -1..=1 {
                    self.canvas.rect(x + d * s, y + d * s, s, s, faded);
                    self.canvas.rect(x + d * s, y - d * s, s, s, faded);
                }
            }
        }
        // The free wells on the chart while a condenser is placed.
        if self.mode == Mode::Build(Kind::Condenser) {
            let r = self.minimap_bounds();
            let chart = self.chart();
            for well in free_wells(&self.world) {
                let (x, y) = chart.plot(&self.world, well);
                let x = x.clamp(r.x + 4 * s, r.x + r.w - 4 * s - 1);
                let y = y.clamp(r.y + 4 * s, r.y + r.h - 4 * s - 1);
                let c = if blink { JADE } else { crate::canvas::WHITE };
                for i in 0..s {
                    self.canvas.frame(
                        x - 4 * s + i,
                        y - 4 * s + i,
                        9 * s - 2 * i,
                        9 * s - 2 * i,
                        c,
                    );
                }
                self.canvas.rect(x - s, y - s, 3 * s, 3 * s, INK);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{RED, WHITE};
    use bw_core::Faction;
    use bw_sim::MapId;
    use std::path::PathBuf;

    fn game(faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-t12field-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.map = MapId::Confluence;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game
    }

    fn spawn(g: &mut Game, owner: u8, kind: Kind, cell: (i32, i32)) -> EntityId {
        let id = g
            .world
            .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Idle;
        }
        id
    }

    fn entity(g: &mut Game, id: EntityId) -> &mut bw_sim::Entity {
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == id)
            .expect("entity")
    }

    fn deploy(g: &mut Game, id: EntityId) {
        let e = entity(g, id);
        e.deployed = true;
        e.deploy_remaining = 0;
    }

    fn hq(g: &Game) -> (EntityId, (i32, i32)) {
        let e = own_hq(&g.world).expect("hq");
        (e.id, e.pos.cell_xy())
    }

    /// Step the nudges alone, tick by tick, as the game does after each
    /// authoritative tick.
    fn observe_for(g: &mut Game, ticks: u64) {
        for _ in 0..ticks {
            g.world.tick += 1;
            let stages = g.resource_stages.clone();
            g.ux_nudges.observe(&g.world, &mut g.ux.alerts, &stages);
        }
    }

    fn card(g: &Game, kind: AlertKind) -> Option<&FieldAlert> {
        g.ux.alerts.entries.iter().find(|a| a.kind == kind)
    }

    #[test]
    fn own_reach_rings_are_gold_and_only_for_a_few_selected_guns() {
        let mut g = game(Faction::Assembly);
        let (_, (hx, hy)) = hq(&g);
        let loom = spawn(&mut g, 0, Kind::Loom, (hx + 8, hy + 8));
        g.selected = vec![loom];
        let plan = g.reach_ring_plan();
        assert_eq!(plan.len(), 1);
        assert!(plan[0].own);
        assert_eq!(plan[0].color, OWN_RING);
        assert_ne!(
            plan[0].color, RED,
            "an own ring never wears the enemy's red"
        );
        assert_eq!(plan[0].reach, spec(Kind::Loom).range);
        assert_eq!(plan[0].blind, LOOM_MIN_RANGE);
        // Eight selected Looms draw no ring each (trial 12, f0111): only
        // the one under the pointer shows its reach.
        let mut looms = vec![loom];
        for i in 0..7 {
            looms.push(spawn(&mut g, 0, Kind::Loom, (hx + 9 + i, hy + 8)));
        }
        g.selected = looms.clone();
        assert!(g.reach_ring_plan().is_empty());
        g.hovered = Some(looms[3]);
        let plan = g.reach_ring_plan();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].pos, Pos::cell(hx + 11, hy + 8));
        // Three selected still ring each, without the blind zones' clutter.
        g.hovered = None;
        g.selected = looms[..3].to_vec();
        let plan = g.reach_ring_plan();
        assert_eq!(plan.len(), 3);
        assert!(plan.iter().all(|r| r.blind == 0 && r.color == OWN_RING));
        // Nothing selected, nothing hovered: no ring at all.
        g.selected.clear();
        assert!(g.reach_ring_plan().is_empty());
    }

    #[test]
    fn enemy_reach_shows_under_the_pointer_and_faintly_over_selected_machines() {
        let mut g = game(Faction::Union);
        let (_, (hx, hy)) = hq(&g);
        let riveter = spawn(&mut g, 0, Kind::Riveter, (hx + 8, hy + 8));
        let enemy = spawn(&mut g, 1, Kind::Loom, (hx + 16, hy + 8));
        deploy(&mut g, enemy);
        assert!(g.world.entity_visible(0, enemy));
        // Nothing of ours selected: the enemy ring stays off.
        assert!(g.reach_ring_plan().is_empty());
        g.selected = vec![riveter];
        let plan = g.reach_ring_plan();
        assert_eq!(plan.len(), 1);
        let hue = crate::seats::seat_colour(&g.world, 1);
        assert!(!plan[0].own);
        assert_eq!(
            plan[0].color,
            with_alpha(hue, ENEMY_RING_ALPHA),
            "faint, in its owner's colour"
        );
        // Under the pointer it is drawn whole.
        g.hovered = Some(enemy);
        assert_eq!(g.reach_ring_plan()[0].color, hue);
        // A packed enemy Loom: none.
        g.hovered = None;
        entity(&mut g, enemy).deployed = false;
        assert!(g.reach_ring_plan().is_empty());
    }

    #[test]
    fn a_deployed_gun_out_of_reach_of_a_near_enemy_is_marked_after_a_spell() {
        let mut g = game(Faction::Assembly);
        let (_, (hx, hy)) = hq(&g);
        let loom = spawn(&mut g, 0, Kind::Loom, (hx + 8, hy + 8));
        deploy(&mut g, loom);
        // A nest one cell beyond the Loom's reach of seven (f0187).
        let nest = spawn(&mut g, 1, Kind::Tower, (hx + 16, hy + 8));
        assert!(g.world.entity_visible(0, nest));
        observe_for(&mut g, 1);
        assert!(
            reach_misses(&g.world, &g.ux_nudges).is_empty(),
            "not at once"
        );
        observe_for(&mut g, STRANDED_GRACE);
        let misses = reach_misses(&g.world, &g.ux_nudges);
        assert_eq!(misses.len(), 1);
        assert_eq!(misses[0].gun, loom);
        assert_eq!(misses[0].short_cells(), 1);
        // An enemy inside the reach clears it.
        let riveter = spawn(&mut g, 1, Kind::Riveter, (hx + 13, hy + 8));
        observe_for(&mut g, 1);
        assert!(reach_misses(&g.world, &g.ux_nudges).is_empty());
        g.world.entities.retain(|e| e.id != riveter);
        // An order on a target out of reach is marked at once.
        entity(&mut g, loom).order = Order::Attack { target: nest };
        assert_eq!(reach_misses(&g.world, &g.ux_nudges).len(), 1);
        // A packed Loom is not a stranded gun.
        entity(&mut g, loom).deployed = false;
        assert!(reach_misses(&g.world, &g.ux_nudges).is_empty());
        // Nothing near: nothing to mark.
        deploy(&mut g, loom);
        entity(&mut g, loom).order = Order::Idle;
        g.world.entities.retain(|e| e.id != nest);
        observe_for(&mut g, STRANDED_GRACE + 1);
        assert!(reach_misses(&g.world, &g.ux_nudges).is_empty());
    }

    #[test]
    fn the_miss_is_drawn_and_named_under_the_pointer() {
        let mut g = game(Faction::Assembly);
        let (_, (hx, hy)) = hq(&g);
        let loom = spawn(&mut g, 0, Kind::Loom, (hx + 8, hy + 8));
        deploy(&mut g, loom);
        let nest = spawn(&mut g, 1, Kind::Tower, (hx + 16, hy + 8));
        entity(&mut g, loom).order = Order::Attack { target: nest };
        g.camera.center(Pos::cell(hx + 12, hy + 8));
        g.field_tips.clear();
        g.draw_reach_misses();
        let tip = g
            .field_tips
            .iter()
            .find(|t| t.card.title == "OUT OF REACH")
            .expect("a tip at the end of the reach");
        assert_eq!(
            tip.card.body,
            vec!["1 cell short. Pack and close in.".to_string()]
        );
    }

    #[test]
    fn full_pressure_raises_a_card_that_selects_the_hq_and_comes_back_later() {
        let mut g = game(Faction::Union);
        let (hq_id, _) = hq(&g);
        g.world.tick = 3 * 60 * 30;
        let cap = g.world.players[0].pressure_cap;
        g.world.players[0].pressure = cap;
        observe_for(&mut g, PRESSURE_GRACE);
        assert!(card(&g, AlertKind::PressureFull).is_none(), "a grace first");
        observe_for(&mut g, 1);
        let first = card(&g, AlertKind::PressureFull).expect("the card").clone();
        assert_eq!(first.label(), "PRESSURE FULL");
        assert!(!first.kind.urgent());
        assert_eq!(
            nudge_icon(first.kind, Faction::Union).as_deref(),
            Some("ui_cmd_reclaim")
        );
        // Clicking it selects the HQ, whose card has RECLAIM and VENT.
        g.selected.clear();
        g.focus_alert(Some(first.id));
        assert_eq!(g.selected, vec![hq_id]);
        // It does not repeat inside its interval, and returns after it.
        observe_for(&mut g, PRESSURE_REPEAT - 1);
        assert!(
            g.ux.alerts
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::PressureFull)
                .all(|a| a.tick == first.tick)
        );
        observe_for(&mut g, 1);
        assert_eq!(
            card(&g, AlertKind::PressureFull).expect("again").tick,
            g.world.tick
        );
        // Spent below the cap: the card goes.
        g.world.players[0].pressure = cap - 100;
        observe_for(&mut g, 1);
        assert!(card(&g, AlertKind::PressureFull).is_none());
    }

    #[test]
    fn reclaim_is_lit_on_the_hq_card_while_pressure_is_full() {
        let mut g = game(Faction::Union);
        let (hq_id, (hx, hy)) = hq(&g);
        g.selected = vec![hq_id];
        g.camera.center(Pos::cell(hx, hy));
        let cap = g.world.players[0].pressure_cap;
        g.world.players[0].pressure = cap;
        g.world.tick = 30 * 60; // a lit phase of the pulse
        g.render();
        let s = g.ui_scale();
        let b = g
            .buttons
            .iter()
            .find(|b| b.action == Action::Reclaim)
            .cloned()
            .expect("RECLAIM on the HQ card");
        assert_eq!(
            g.canvas.get(b.x - s, b.y + b.h / 2),
            Some(GOLD),
            "RECLAIM framed in gold"
        );
        g.world.players[0].pressure = 0;
        g.render();
        assert_ne!(g.canvas.get(b.x - s, b.y + b.h / 2), Some(GOLD));
    }

    #[test]
    fn nudge_cards_carry_a_picture_that_exists_for_every_side() {
        let g = game(Faction::Union);
        let atlas = g.atlas.as_ref().expect("atlas");
        for faction in Faction::ALL {
            for kind in [
                AlertKind::PressureFull,
                AlertKind::WrecksOut,
                AlertKind::ProducersIdle,
            ] {
                let key = nudge_icon(kind, faction).expect("a picture");
                assert!(atlas.sprites.contains_key(&key), "{key}");
            }
        }
    }

    #[test]
    fn home_wrecks_running_dry_raise_one_card_that_jumps_to_a_live_wreck() {
        let mut g = game(Faction::Union);
        g.world.tick = 30 * 30 - 1;
        observe_for(&mut g, 1);
        assert!(
            home_live_wrecks(&g.world, &g.resource_stages) > 0,
            "home wrecks at the start"
        );
        assert!(card(&g, AlertKind::WrecksOut).is_none());
        // Empty every home wreck.
        let home = own_hq(&g.world).expect("hq").pos;
        let limit = i64::from(HOME_WRECK_CELLS * FP).pow(2);
        for r in &mut g.world.map.resources {
            if home.distance_sq(r.pos) <= limit {
                r.remaining = 0;
            }
        }
        observe_for(&mut g, 30);
        let wrecks = card(&g, AlertKind::WrecksOut).expect("WRECKS OUT").clone();
        assert_eq!(wrecks.label(), "WRECKS OUT");
        // Once only while they stay dry.
        observe_for(&mut g, 90);
        assert_eq!(
            g.ux.alerts
                .entries
                .iter()
                .filter(|a| a.kind == AlertKind::WrecksOut)
                .count(),
            1
        );
        assert_eq!(
            card(&g, AlertKind::WrecksOut).expect("card").tick,
            wrecks.tick
        );
        // The click goes to the nearest live wreck a worker can reach.
        let target = g
            .nearest_reachable_wreck()
            .expect("a live wreck further out");
        let r = g
            .world
            .map
            .resources
            .iter()
            .find(|r| r.pos == target)
            .expect("wreck");
        assert!(r.remaining > 0);
        // The card stands there, so its echo on the chart pings the wreck.
        assert_eq!(wrecks.pos, target);
        g.focus_alert(Some(wrecks.id));
        let mut there = g.camera;
        there.center(target);
        assert_eq!((g.camera.x, g.camera.y), (there.x, there.y));
    }

    #[test]
    fn spent_wreck_beds_fade_on_the_chart() {
        let mut g = game(Faction::Union);
        let home = own_hq(&g.world).expect("hq").pos;
        let bed = g
            .world
            .map
            .resources
            .iter()
            .filter(|r| !r.scrap)
            .min_by_key(|r| r.pos.distance_sq(home))
            .map(|r| (r.id, r.pos))
            .expect("a home bed");
        let chart = g.chart();
        let (x, y) = chart.plot(&g.world, bed.1);
        let rock = crate::minimap_chart::ROCK;
        g.render();
        assert_eq!(g.canvas.get(x, y), Some(rock), "a live bed in ink");
        for r in &mut g.world.map.resources {
            if r.id == bed.0 {
                r.remaining = 0;
            }
        }
        g.render();
        assert_eq!(g.resource_stages.get(&bed.0), Some(&3));
        assert_eq!(
            g.canvas.get(x, y),
            Some(crate::minimap_chart::PAPER_AGED),
            "a spent bed in pale ink"
        );
    }

    #[test]
    fn spent_wrecks_lie_flat_and_grey() {
        let g = game(Faction::Union);
        let atlas = g.atlas.as_ref().expect("atlas");
        let key = ["salvage_stage_3", "salvage"]
            .into_iter()
            .find(|k| atlas.sprites.contains_key(*k))
            .expect("wreck art");
        let bounds = |c: &Canvas| {
            let (mut top, mut bottom, mut n) = (i32::MAX, i32::MIN, 0);
            for y in 0..c.height() as i32 {
                for x in 0..c.width() as i32 {
                    if c.get(x, y) != Some([0, 0, 0, 255]) {
                        top = top.min(y);
                        bottom = bottom.max(y);
                        n += 1;
                    }
                }
            }
            (bottom - top, n)
        };
        let mut live = Canvas::new(120, 120);
        live.clear([0, 0, 0, 255]);
        atlas.draw(&mut live, key, 60, 80, false);
        let mut spent = Canvas::new(120, 120);
        spent.clear([0, 0, 0, 255]);
        assert!(draw_spent(atlas, &mut spent, key, 60, 80, false));
        let (live_h, _) = bounds(&live);
        let (spent_h, count) = bounds(&spent);
        assert!(count > 0);
        assert!(spent_h < live_h, "flattened: {spent_h} < {live_h}");
        // Grey: no texel keeps a strong hue.
        for y in 0..120 {
            for x in 0..120 {
                let c = spent.get(x, y).expect("pixel");
                let (lo, hi) = (c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
                assert!(hi - lo <= 24, "{c:?}");
            }
        }
    }

    #[test]
    fn idle_works_raise_a_card_while_a_machine_is_affordable() {
        let mut g = game(Faction::Compact);
        let (_, (hx, hy)) = hq(&g);
        let works = spawn(&mut g, 0, Kind::Works, (hx + 8, hy - 6));
        g.world.players[0].salvage = 1000;
        g.world.players[0].pressure = 100;
        g.world.tick = 5 * 60 * 30;
        observe_for(&mut g, PRODUCER_IDLE);
        assert!(card(&g, AlertKind::ProducersIdle).is_none());
        observe_for(&mut g, 1);
        let idle = card(&g, AlertKind::ProducersIdle).expect("card").clone();
        assert_eq!(idle.count, 1);
        // The side's own name: the Compact's Glassworks.
        let name = crate::ux::building_name(Kind::Works, Faction::Compact);
        assert_eq!(idle.label(), format!("{name} IDLE"));
        g.selected.clear();
        g.focus_alert(Some(idle.id));
        assert_eq!(g.selected, vec![works]);
        // Broke: no card.
        g.world.players[0].salvage = 0;
        observe_for(&mut g, 1);
        assert!(card(&g, AlertKind::ProducersIdle).is_none());
        // Affordable again inside the interval: not raised again.
        g.world.players[0].salvage = 1000;
        observe_for(&mut g, 30);
        assert!(
            card(&g, AlertKind::ProducersIdle).is_none(),
            "one card a spell"
        );
        // A queue ends the spell.
        entity(&mut g, works).queue.push(bw_sim::Production {
            kind: Kind::Brander,
            remaining: 100,
            started: true,
            cost_salvage: 0,
            cost_pressure: 0,
        });
        observe_for(&mut g, 1);
        assert!(g.ux_nudges.idle_card.is_none());
    }

    #[test]
    fn placing_a_condenser_marks_every_free_well_and_only_those() {
        let mut g = game(Faction::Union);
        let wells = free_wells(&g.world);
        assert_eq!(wells.len(), g.world.map.wells.len());
        let taken = wells[0];
        let (x, y) = taken.cell_xy();
        spawn(&mut g, 0, Kind::Condenser, (x, y));
        let after = free_wells(&g.world);
        assert_eq!(after.len(), wells.len() - 1);
        assert!(!after.contains(&taken));
        // The chart marks them while placing.
        g.mode = Mode::Build(Kind::Condenser);
        g.world.tick = 0;
        g.render();
        let chart = g.chart();
        let s = g.ui_scale();
        let r = g.minimap_bounds();
        let (px, py) = chart.plot(&g.world, after[0]);
        let px = px.clamp(r.x + 4 * s, r.x + r.w - 4 * s - 1);
        let py = py.clamp(r.y + 4 * s, r.y + r.h - 4 * s - 1);
        assert_eq!(g.canvas.get(px - 4 * s, py), Some(JADE));
    }

    #[test]
    fn tidal_ground_and_the_sluice_approach_are_refused_ground() {
        let g = game(Faction::Union);
        let map = &g.world.map;
        let mut tidal = 0;
        for y in 0..i32::from(map.height) {
            for x in 0..i32::from(map.width) {
                let t = map.terrain(x, y);
                if t.is_tidal() && !matches!(g.world.depth_at(x, y), Some(bw_sim::Depth::Deep)) {
                    assert!(refused_ground(&g.world, x, y));
                    tidal += 1;
                }
                if refused_ground(&g.world, x, y) {
                    // Everything hatched really refuses a building.
                    assert!(
                        g.placement_cell_refusal(Pos::cell(x, y), false).is_some(),
                        "({x}, {y})"
                    );
                }
            }
        }
        assert!(tidal > 0);
        let (hx, hy) = own_hq(&g.world).expect("hq").pos.cell_xy();
        assert!(
            !refused_ground(&g.world, hx + 6, hy + 6),
            "home salt takes a building"
        );
        // The hatch is a light one: a third of the diamond at most.
        let mut c = Canvas::new(40, 20);
        c.clear([0, 0, 0, 255]);
        hatch_cell(&mut c, 20, 10, [255, 0, 0, 255]);
        let lit = (0..20)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&(x, y)| c.get(x, y) == Some([255, 0, 0, 255]))
            .count();
        assert!(lit > 20 && lit < 16 * 8 * 2 / 3, "{lit}");
    }

    #[test]
    fn heap_pips_step_aside_while_a_fight_is_on_beside_them() {
        let mut g = game(Faction::Union);
        let (_, (hx, hy)) = hq(&g);
        let at = Pos::cell(hx + 6, hy + 6);
        assert!(!fight_near(&g.world, &[], at));
        assert!(fight_near(&g.world, &[Pos::cell(hx + 8, hy + 6)], at));
        assert!(!fight_near(&g.world, &[Pos::cell(hx + 20, hy + 6)], at));
        let brander = spawn(&mut g, 1, Kind::Brander, (hx + 8, hy + 6));
        assert!(g.world.entity_visible(0, brander));
        assert!(fight_near(&g.world, &[], at));
        // An enemy worker is no fight.
        g.world.entities.retain(|e| e.id != brander);
        spawn(&mut g, 1, Kind::Raker, (hx + 7, hy + 6));
        assert!(!fight_near(&g.world, &[], at));
    }

    #[test]
    fn machines_behind_the_station_keep_a_ghost_through_its_rails() {
        let mut g = game(Faction::Union);
        let pieces = g.station3_pieces().expect("the three-arm station");
        let atlas = g.atlas.as_ref().expect("atlas");
        let key = Kind::Riveter.asset(Faction::Union).to_string();
        assert!(atlas.sprites.contains_key(&key), "{key}");
        let (x, y) = (200, 200);
        let parts: Vec<(String, i32, i32)> = pieces
            .iter()
            .map(|(k, dx, dy)| (k.clone(), x + dx, y + dy))
            .collect();
        let mut canvas = Canvas::new(400, 400);
        canvas.clear([0, 0, 0, 255]);
        let unit = GhostUnit {
            key: key.clone(),
            x,
            y: y - 20,
            scale: PixelScale::ONE,
            tint: RED,
        };
        let drawn = ghost_behind_station(atlas, &mut canvas, &[unit], &parts);
        assert!(drawn > 0);
        let changed = canvas
            .pixels
            .chunks_exact(4)
            .filter(|p| p[..3] != [0, 0, 0])
            .count();
        assert!(changed > 20, "{changed}");
        // Far from the station, nothing.
        let far = GhostUnit {
            key,
            x: 5000,
            y: 5000,
            scale: PixelScale::ONE,
            tint: WHITE,
        };
        assert_eq!(ghost_behind_station(atlas, &mut canvas, &[far], &parts), 0);
    }
}

/// Review scenes for trial 12's field fixes, written as quick saves the
/// headless game loads with F9 (both before and after the fixes, since the
/// world format is the simulation's): `BW_T12_SCENES=DIR cargo test -p
/// bw_desktop write_trial12_scenes -- --ignored`. Each scene's directory is
/// a `--data-dir`; `points.json` beside the save gives the frame points the
/// review clicks and hovers, at 1280x720.
#[cfg(test)]
mod review_scenes {
    use super::*;
    use bw_core::Faction;
    use bw_sim::MapId;
    use std::path::{Path, PathBuf};

    fn scene(faction: Faction, dir: &Path) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut game = Game::new_with_data_dir(base, dir.to_path_buf());
        game.faction = faction;
        game.map = MapId::Confluence;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game
    }

    fn spawn(g: &mut Game, owner: u8, kind: Kind, cell: (i32, i32)) -> EntityId {
        let id = g
            .world
            .spawn_for_tests(owner, kind, Pos::cell(cell.0, cell.1));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Hold;
        }
        id
    }

    fn deploy(g: &mut Game, id: EntityId) {
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.deployed = true;
            e.deploy_remaining = 0;
            e.keep_deployed = true;
        }
    }

    /// Save the scene and write where things stand on a 1280x720 frame
    /// after it loads.
    fn save(g: &mut Game, dir: &Path, focus: Option<Pos>, points: &[(&str, Pos)]) {
        std::fs::create_dir_all(dir.join("saves")).unwrap();
        g.world.save(dir.join("saves/quick-save.json")).unwrap();
        let mut probe = scene(g.faction, dir);
        // As the headless game sets its view up.
        probe.zoom = crate::zoom::Zoom::from_preference(probe.ux.preferences.world_zoom);
        probe.resize_view(1280, 720);
        probe.load_match();
        let mut out = serde_json::Map::new();
        // The pan that brings the focus to the middle of the field.
        let view = probe.world_view();
        let pan = focus.map_or((0, 0), |focus| {
            let (x, y) = view.screen(probe.camera.project(focus));
            let (cx, cy) = view.center();
            (x - cx, y - cy)
        });
        probe.pan(pan.0, pan.1);
        out.insert("pan".into(), serde_json::json!([pan.0, pan.1]));
        let view = probe.world_view();
        for (name, pos) in points {
            let (x, y) = view.screen(probe.camera.project(*pos));
            out.insert((*name).into(), serde_json::json!([x, y]));
        }
        std::fs::write(
            dir.join("points.json"),
            serde_json::to_vec_pretty(&out).unwrap(),
        )
        .unwrap();
    }

    fn hq_cell(g: &Game) -> (i32, i32) {
        own_hq(&g.world).unwrap().pos.cell_xy()
    }

    #[test]
    #[ignore]
    fn write_trial12_scenes() {
        let Ok(root) = std::env::var("BW_T12_SCENES") else {
            return;
        };
        let root = PathBuf::from(root);

        // Rings and reach: eight kept-deployed Looms, a nest a cell beyond
        // the front Loom's reach, an enemy deployed Loom to the side.
        let dir = root.join("rings");
        let mut g = scene(Faction::Assembly, &dir);
        let (hx, hy) = hq_cell(&g);
        let mut looms = Vec::new();
        for i in 0..4 {
            for j in 0..2 {
                let id = spawn(&mut g, 0, Kind::Loom, (hx + 8 + i, hy + 7 + j * 2));
                deploy(&mut g, id);
                looms.push(id);
            }
        }
        spawn(&mut g, 1, Kind::Tower, (hx + 19, hy + 8));
        let enemy = spawn(&mut g, 2, Kind::Heliostat, (hx + 12, hy + 17));
        deploy(&mut g, enemy);
        let front = Pos::cell(hx + 11, hy + 9);
        save(
            &mut g,
            &dir,
            Some(Pos::cell(hx + 14, hy + 10)),
            &[
                ("front_loom", front),
                ("box_from", Pos::cell(hx + 7, hy + 6)),
                ("box_to", Pos::cell(hx + 12, hy + 10)),
                ("nest", Pos::cell(hx + 19, hy + 8)),
            ],
        );

        // The station: our machines behind its rails and wheels, a fight
        // at its salvage.
        let dir = root.join("station");
        let mut g = scene(Faction::Union, &dir);
        let gate = g.world.map.gate_pos.cell_xy();
        for (dx, dy) in [(-2, -3), (-1, -3), (0, -3), (-3, -1), (-3, 0), (1, -2)] {
            spawn(&mut g, 0, Kind::Riveter, (gate.0 + dx, gate.1 + dy));
        }
        for (dx, dy) in [(2, 1), (3, 2)] {
            spawn(&mut g, 2, Kind::Brander, (gate.0 + dx, gate.1 + dy));
        }
        let next = g
            .world
            .map
            .resources
            .iter()
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            + 1;
        for (i, (dx, dy)) in [(1, 0), (2, 0), (0, 1)].into_iter().enumerate() {
            g.world.map.resources.push(bw_sim::Resource {
                id: next + i as u32,
                pos: Pos::cell(gate.0 + dx, gate.1 + dy),
                remaining: 400,
                kind: bw_sim::ResourceKind::Salvage,
                scrap: true,
            });
        }
        g.world.gate.owner = Some(0);
        save(
            &mut g,
            &dir,
            Some(Pos::cell(gate.0, gate.1)),
            &[("gate", Pos::cell(gate.0, gate.1))],
        );

        // The sluice approach: a worker and an escort by the gate, for a
        // nest placed on ground that refuses it.
        let dir = root.join("approach");
        let mut g = scene(Faction::Union, &dir);
        let gate = g.world.map.gate_pos.cell_xy();
        for e in g.world.entities.iter_mut() {
            if e.owner == 0 && e.kind.is_worker() {
                e.order = Order::Hold;
            }
        }
        let hook = spawn(&mut g, 0, Kind::Hook, (gate.0 - 5, gate.1 - 2));
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == hook) {
            e.order = Order::Idle;
        }
        for (dx, dy) in [(-4, -4), (4, -4), (-4, 4), (4, 4)] {
            spawn(&mut g, 0, Kind::Riveter, (gate.0 + dx, gate.1 + dy));
        }
        g.world.gate.owner = Some(0);
        save(
            &mut g,
            &dir,
            Some(Pos::cell(gate.0, gate.1)),
            &[
                ("gate", Pos::cell(gate.0, gate.1)),
                ("near", Pos::cell(gate.0 - 3, gate.1 + 1)),
            ],
        );

        // Placement: one idle worker at home, for a condenser and a nest.
        let dir = root.join("placement");
        let mut g = scene(Faction::Union, &dir);
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .unwrap();
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == worker) {
            e.order = Order::Idle;
        }
        let (hx, hy) = hq_cell(&g);
        let layout = g.world.map.layout();
        let mouth = layout
            .arm_mouths(g.world.arms_of(0)[0])
            .into_iter()
            .min_by_key(|m| m.distance_sq(Pos::cell(hx, hy)))
            .unwrap();
        // Scouts along the lane to the mouth, so its ground is seen.
        let (mx, my) = mouth.cell_xy();
        for k in 1..=4 {
            let (sx, sy) = (hx + (mx - hx) * k / 4, hy + (my - hy) * k / 4);
            spawn(&mut g, 0, Kind::Riveter, (sx, sy));
        }
        save(
            &mut g,
            &dir,
            None,
            &[("home", Pos::cell(hx + 5, hy + 7)), ("mouth", mouth)],
        );

        // Pressure at its cap six minutes in.
        let dir = root.join("pressure");
        let mut g = scene(Faction::Union, &dir);
        g.world.tick = 6 * 60 * 30;
        let cap = g.world.players[0].pressure_cap;
        g.world.players[0].pressure = cap;
        g.world.players[0].salvage = 40;
        save(&mut g, &dir, None, &[]);

        // Home wrecks: all spent but one with a last load, and a worker on it.
        let dir = root.join("wrecks");
        let mut g = scene(Faction::Union, &dir);
        g.world.tick = 8 * 60 * 30;
        let home = own_hq(&g.world).unwrap().pos;
        let limit = i64::from(HOME_WRECK_CELLS * FP).pow(2);
        let mut last = None;
        let mut near: Vec<(i64, u32)> = g
            .world
            .map
            .resources
            .iter()
            .filter(|r| r.pos.distance_sq(home) <= limit)
            .map(|r| (r.pos.distance_sq(home), r.id))
            .collect();
        near.sort();
        for r in &mut g.world.map.resources {
            if r.pos.distance_sq(home) <= limit {
                if Some(r.id) == near.first().map(|n| n.1) {
                    r.remaining = 4;
                    last = Some((r.id, r.pos));
                } else {
                    r.remaining = 0;
                }
            }
        }
        let (rid, rpos) = last.unwrap();
        for e in g.world.entities.iter_mut() {
            if e.owner == 0 && e.kind.is_worker() {
                e.order = Order::Gather { resource: rid };
            }
        }
        save(&mut g, &dir, Some(rpos), &[("last_wreck", rpos)]);

        // Idle Works: three with empty queues and salvage to spend.
        let dir = root.join("works");
        let mut g = scene(Faction::Assembly, &dir);
        g.world.tick = 16 * 60 * 30;
        let (hx, hy) = hq_cell(&g);
        for (dx, dy) in [(7, -4), (7, 0), (-6, 6)] {
            spawn(&mut g, 0, Kind::Works, (hx + dx, hy + dy));
        }
        g.world.players[0].salvage = 1500;
        g.world.players[0].pressure = 200;
        save(&mut g, &dir, None, &[]);
    }
}
