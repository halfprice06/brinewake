//! Bounded, observed-only traces left by a crossing.
//!
//! Traces are presentation state.  They never participate in simulation
//! rules, entity selection, replay hashes, or pathing.  The state is intended
//! to live in the desktop UI sidecar next to a hash-matched world save.  A
//! trace is only born from information available to player 0 at the time the
//! event is observed; no current hidden entity lookup is used for shots or
//! deaths.

use crate::canvas::{Atlas, Canvas};
use bw_core::{Camera, EntityId, FP, Pos, Terrain};
use bw_sim::{Event, EventKind, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum number of retained trace marks in the presentation sidecar.
///
/// Ninety-six is deliberately below the number of world entities permitted by
/// the simulation.  It keeps a long match readable and bounds both JSON size
/// and the per-frame clipping work.
pub const MAX_TRACES: usize = 96;
/// Maximum number of deployment states retained by the explicit observer.
/// These are IDs and last-seen values, not world entities or gameplay state.
pub const MAX_DEPLOYMENT_OBSERVATIONS: usize = 128;

const JACK_WASH_TICKS: u64 = 150;
const RIVET_WASH_TICKS: u64 = 72;
const BINDING_WASH_TICKS: u64 = 48;
const GLAZE_WASH_TICKS: u64 = 72;
const JACK_SILT_TICKS: u64 = 180;
const LIGHT_SILT_TICKS: u64 = 90;
const WASH_STAGGER_TICKS: u64 = 6;
const MAX_SCHEDULED_WASH_AHEAD: u64 = WASH_STAGGER_TICKS * 3;
const MAX_VISIBILITY_CELLS: usize = 1024;

/// The four authored atlas families used by the trace pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    /// The hard footprint left by a specialist jack settling or packing.
    JackFoot,
    /// A spent Union rivet at an observed firing location.
    Rivet,
    /// A torn Assembly binding at an observed death location.
    Binding,
    /// Compact glass fused into the lane where an observed shot landed.
    Glaze,
    /// Unknown future values are accepted by serde and dropped by
    /// sanitization so one malformed mark cannot discard the whole sidecar.
    #[serde(other)]
    Invalid,
}

impl TraceKind {
    fn asset_name(self) -> &'static str {
        match self {
            Self::JackFoot => "jack",
            Self::Rivet => "rivet",
            Self::Binding => "binding",
            Self::Glaze => "glaze",
            Self::Invalid => "invalid",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::JackFoot => 0,
            Self::Rivet => 1,
            Self::Binding => 2,
            Self::Glaze => 3,
            Self::Invalid => 4,
        }
    }

    /// Hard jack impressions stay readable through a longer wash than the
    /// light rivet, binding and glaze marks.
    fn wash_ticks(self) -> u64 {
        match self {
            Self::JackFoot => JACK_WASH_TICKS,
            Self::Rivet => RIVET_WASH_TICKS,
            Self::Binding => BINDING_WASH_TICKS,
            Self::Glaze => GLAZE_WASH_TICKS,
            Self::Invalid => 0,
        }
    }

    fn silt_ticks(self) -> u64 {
        match self {
            Self::JackFoot => JACK_SILT_TICKS,
            Self::Rivet | Self::Binding | Self::Glaze => LIGHT_SILT_TICKS,
            Self::Invalid => 0,
        }
    }
}

/// One observed mark.  `lane` is stored so a future map change cannot make a
/// historical mark silently migrate to another material; sanitization checks
/// it against the current map before retaining the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trace {
    pub kind: TraceKind,
    pub pos: Pos,
    pub lane: Terrain,
    pub born_tick: u64,
    /// Tick at which water starts washing this mark.  A `None` value is a
    /// fresh dry-lane mark.
    #[serde(default)]
    pub wash_start: Option<u64>,
    /// Tick at which a newly dry lane settles the mark into a small silt
    /// remnant.  This is also presentation state and may be absent.
    #[serde(default)]
    pub dry_tick: Option<u64>,
}

/// Last visible observation for one specialist. Requiring a consecutive tick
/// prevents a hidden deploy/pack toggle from becoming a mark when the unit is
/// seen again later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentObservation {
    pub deployed: bool,
    pub last_seen_tick: u64,
}

/// Sidecar state for observed tidal traces.
///
/// The deployment cache is intentionally part of the serialized state.  If a
/// saved specialist was already deployed, loading the save must not treat its
/// unchanged state as a new deployment and invent a mark.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceState {
    pub traces: Vec<Trace>,
    #[serde(default)]
    pub observed_deployed: BTreeMap<EntityId, DeploymentObservation>,
}

impl TraceState {
    /// Remove malformed, future, expired, or out-of-map marks and enforce the
    /// bounded sidecar contract.  Call this after loading a hash-matched UI
    /// sidecar and before saving it again.
    pub fn sanitize(&mut self, world: &World) {
        let tick = world.tick;
        self.traces.retain_mut(|trace| {
            if matches!(trace.kind, TraceKind::Invalid)
                || trace.born_tick > tick
                || !trace.pos.valid(world.map.width, world.map.height)
            {
                return false;
            }
            let (x, y) = trace.pos.cell_xy();
            if !trace.lane.is_lane() || world.map.terrain(x, y) != trace.lane {
                return false;
            }

            // A short future wash clock is valid for deterministic staggered
            // starts. Far-future clocks are malformed and become fresh marks;
            // a mark whose birth itself is in the future was dropped above.
            if trace
                .wash_start
                .is_some_and(|start| start > tick.saturating_add(MAX_SCHEDULED_WASH_AHEAD))
            {
                trace.wash_start = None;
            }
            if trace.dry_tick.is_some_and(|dry| dry > tick) {
                trace.dry_tick = None;
            }
            if trace
                .wash_start
                .is_some_and(|start| start < trace.born_tick)
            {
                trace.wash_start = Some(trace.born_tick);
            }
            if trace.dry_tick.is_some_and(|dry| dry < trace.born_tick) {
                trace.dry_tick = Some(trace.born_tick);
            }

            let expiry = trace
                .dry_tick
                .map(|dry| dry.saturating_add(trace.kind.silt_ticks()))
                .or_else(|| {
                    trace.wash_start.map(|start| {
                        start
                            .saturating_add(trace.kind.wash_ticks())
                            .saturating_add(trace.kind.silt_ticks())
                    })
                });
            expiry.is_none_or(|until| tick < until)
        });
        self.sort_and_cap();

        // Zero is not an allocated simulation ID.  Keeping only the bounded
        // cache makes malformed sidecars cheap to recover from while retaining
        // recent last-seen IDs; a hidden toggle cannot be inferred from absence.
        self.observed_deployed
            .retain(|id, observation| *id != 0 && observation.last_seen_tick <= tick);
        while self.observed_deployed.len() > MAX_DEPLOYMENT_OBSERVATIONS {
            let Some(id) = self.observed_deployed.keys().next_back().copied() else {
                break;
            };
            self.observed_deployed.remove(&id);
        }
    }

    /// Observe the current deployed state of living specialists that player 0
    /// can see.  A first sighting only seeds the cache.  A transition detected
    /// while visible leaves one jack-foot trace at the entity's current
    /// position. Unseen entities are skipped; an observation that misses one
    /// authoritative tick expires and a later re-entry only seeds state.
    pub fn observe_deployments(&mut self, world: &World) {
        let mut visible = Vec::new();
        for entity in &world.entities {
            if entity.hp <= 0
                || entity.build_remaining > 0
                || !matches!(entity.kind, bw_core::Kind::Bulwark | bw_core::Kind::Loom)
                || !world.entity_visible(0, entity.id)
            {
                continue;
            }
            visible.push((entity.id, entity.deployed, entity.pos));
        }

        for (id, deployed, pos) in visible {
            let previous = self.observed_deployed.get(&id).copied();
            let consecutive = previous.is_some_and(|observation| {
                observation.last_seen_tick.saturating_add(1) == world.tick
            });
            if consecutive && previous.is_some_and(|observation| observation.deployed != deployed) {
                self.push_trace(world, TraceKind::JackFoot, pos);
            }
            self.observed_deployed.insert(
                id,
                DeploymentObservation {
                    deployed,
                    last_seen_tick: world.tick,
                },
            );
        }

        // First sightings only seed the cache. Expire observations after one
        // missed tick so re-entry cannot infer a hidden toggle.
        self.observed_deployed
            .retain(|_, observation| observation.last_seen_tick.saturating_add(1) >= world.tick);
        while self.observed_deployed.len() > MAX_DEPLOYMENT_OBSERVATIONS {
            let Some(id) = self.observed_deployed.keys().next_back().copied() else {
                break;
            };
            self.observed_deployed.remove(&id);
        }
        self.sort_and_cap();
    }

    /// Record one simulation event using the same source-observed predicate as
    /// the desktop event presentation path.  Hidden enemy actions return
    /// without creating, updating, or erasing a mark.
    pub fn record_event(&mut self, world: &World, event: &Event) {
        // A caller may hand us an event from an untrusted sidecar/replay.  A
        // future event is never allowed to seed a future visual mark.
        if event.tick > world.tick {
            return;
        }

        if event.kind == EventKind::GateChanged {
            // Gate state is public.  It may wash already observed marks even
            // when the event has no local owner, but it never consults hidden
            // entity IDs or positions.
            self.apply_gate_change(world);
            return;
        }

        if !event_is_observed(world, event) {
            return;
        }

        match event.kind {
            EventKind::Shot if event_is_faction(world, event, bw_core::Faction::Union) => {
                // Prefer the visible impact point.  If the impact is hidden,
                // fall back to the visible firing point; never ask the world
                // for a target's current position after the event.
                let pos = event
                    .to
                    .filter(|pos| world.visible(0, *pos))
                    .or_else(|| event.from.filter(|pos| world.visible(0, *pos)));
                if let Some(pos) = pos {
                    self.push_trace(world, TraceKind::Rivet, pos);
                }
            }
            EventKind::Shot if event_is_faction(world, event, bw_core::Faction::Compact) => {
                // The glass fuses where the light or lance lands, so only a
                // visible impact point leaves one; the firing point never
                // does.
                if let Some(pos) = event.to.filter(|pos| world.visible(0, *pos)) {
                    self.push_trace(world, TraceKind::Glaze, pos);
                }
            }
            EventKind::Death if event_is_faction(world, event, bw_core::Faction::Assembly) => {
                // Death removes the entity before this presentation hook can
                // run.  The authoritative event position is the only safe
                // historical location and is still fog-checked above.
                if let Some(pos) = event.from.filter(|pos| world.visible(0, *pos)) {
                    self.push_trace(world, TraceKind::Binding, pos);
                }
            }
            _ => {}
        }
    }

    /// Draw observed traces beneath actors.  The pass clips every nontransparent
    /// sprite pixel against the trace's lane and current player-0 visibility,
    /// so a 32x16 asset cannot bridge a bank or reveal a hidden patch.  The
    /// visual clock is supplied by the caller and is never advanced here.
    ///
    /// Returns the number of trace sprites whose visible pixels were drawn.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        atlas: Option<&Atlas>,
        world: &World,
        camera: Camera,
        visual_tick: u64,
    ) -> usize {
        let mut drawn = 0;
        let view_width = i32::try_from(canvas.width()).expect("Canvas width fits i32");
        let view_height = i32::try_from(canvas.height()).expect("Canvas height fits i32");
        let mut visible_cells = BTreeMap::<(i32, i32), bool>::new();
        let mut ordered: Vec<&Trace> = self.traces.iter().collect();
        ordered.sort_by_key(|trace| (trace.pos.y, trace.pos.x, trace.born_tick, trace.kind.rank()));

        for trace in ordered {
            if !trace_drawable(trace, world, visual_tick) {
                continue;
            }
            let (x, y) = camera.project(trace.pos);
            let phase = trace_phase(trace, visual_tick);
            let key = format!("trace_{}_{}", trace.kind.asset_name(), phase);
            let sprite = atlas
                .and_then(|atlas| atlas.sprites.get(&key))
                .filter(|s| s.w == 32 && s.h == 16 && s.anchor_x == 16 && s.anchor_y == 8);
            let (width, height, anchor_x, anchor_y) = sprite
                .map(|sprite| (sprite.w, sprite.h, sprite.anchor_x, sprite.anchor_y))
                .unwrap_or((32, 16, 16, 8));
            let mut copied = false;
            for yy in 0..height {
                for xx in 0..width {
                    let color = if let Some(sprite) = sprite {
                        atlas.and_then(|atlas| atlas.sample(sprite, xx, yy))
                    } else {
                        fallback_pixel(trace.kind, phase, xx as i32, yy as i32)
                    };
                    let Some(color) = color else {
                        continue;
                    };
                    if color[3] == 0 {
                        continue;
                    }
                    let sx = x + xx as i32 - anchor_x;
                    let sy = y + yy as i32 - anchor_y;
                    if !(0..view_width).contains(&sx) || !(0..view_height).contains(&sy) {
                        continue;
                    }
                    if !pixel_is_allowed(world, camera, trace, sx, sy, &mut visible_cells) {
                        continue;
                    }
                    canvas.pixel(sx, sy, color);
                    copied = true;
                }
            }
            if copied {
                drawn += 1;
            }
        }
        drawn
    }

    fn push_trace(&mut self, world: &World, kind: TraceKind, pos: Pos) {
        let Some(lane) = lane_at(world, pos) else {
            // Non-lane positions do not become debris.  This is also the
            // guard that keeps shots near a bank from crossing into scenery.
            return;
        };
        let mut trace = Trace {
            kind,
            pos,
            lane,
            born_tick: world.tick,
            wash_start: None,
            dry_tick: None,
        };
        if lane_is_wet(world, lane) {
            trace.wash_start = Some(world.tick);
        }
        self.traces.push(trace);
        self.sort_and_cap();
    }

    fn apply_gate_change(&mut self, world: &World) {
        // Every lane follows its own arm: water washes the marks on each wet
        // lane and a dry lane settles its washed marks into silt.
        let mut stagger_index = 0u64;
        for trace in &mut self.traces {
            if !trace.lane.is_lane() {
                continue;
            }
            if lane_is_wet(world, trace.lane) {
                // A new flood washes an existing dry/silt trace.  Retain the
                // earliest active wash if it was already in water; reset only
                // a residue that has just become submerged again.
                if trace.dry_tick.take().is_some() {
                    trace.wash_start = Some(world.tick);
                } else if trace.wash_start.is_none() {
                    trace.wash_start =
                        Some(world.tick.saturating_add(
                            (stagger_index % 4).saturating_mul(WASH_STAGGER_TICKS),
                        ));
                }
                stagger_index = stagger_index.saturating_add(1);
            } else if trace.wash_start.is_some() {
                // The paving returns when water drains.  Leave a small silt
                // edge rather than an entity-like wreck.
                trace.dry_tick = Some(world.tick);
            }
        }
        self.sort_and_cap();
    }

    fn sort_and_cap(&mut self) {
        self.traces
            .sort_by_key(|trace| (trace.born_tick, trace.pos.y, trace.pos.x, trace.kind.rank()));
        if self.traces.len() > MAX_TRACES {
            let excess = self.traces.len() - MAX_TRACES;
            self.traces.drain(0..excess);
        }
    }
}

fn event_is_observed(world: &World, event: &Event) -> bool {
    event.player == Some(0)
        || event.entity.is_some_and(|id| world.entity_visible(0, id))
        || event.from.is_some_and(|pos| world.visible(0, pos))
}

fn event_is_faction(world: &World, event: &Event, faction: bw_core::Faction) -> bool {
    event
        .player
        .and_then(|player| world.players.get(player as usize))
        .is_some_and(|player| player.faction == faction)
}

fn lane_at(world: &World, pos: Pos) -> Option<Terrain> {
    if !pos.valid(world.map.width, world.map.height) {
        return None;
    }
    let (x, y) = pos.cell_xy();
    match world.map.terrain(x, y) {
        Terrain::Lane0 | Terrain::Lane1 | Terrain::Lane2 => Some(world.map.terrain(x, y)),
        _ => None,
    }
}

fn lane_is_wet(world: &World, lane: Terrain) -> bool {
    crate::presentation::cell_is_wet(world, lane)
}

fn trace_phase(trace: &Trace, tick: u64) -> u8 {
    if trace.dry_tick.is_some_and(|dry| tick >= dry) {
        return 5;
    }
    let Some(start) = trace.wash_start else {
        return 0;
    };
    let Some(age) = tick.checked_sub(start) else {
        return 0;
    };
    let wash = trace.kind.wash_ticks();
    if age >= wash {
        return 5;
    }
    // Four equal, stagger-safe bands make phases 1..4 visible without tying
    // cosmetics to render frame count.
    ((age.saturating_mul(4) / wash).min(3) + 1) as u8
}

fn trace_drawable(trace: &Trace, world: &World, tick: u64) -> bool {
    if !trace.pos.valid(world.map.width, world.map.height)
        || lane_at(world, trace.pos) != Some(trace.lane)
        || !world.visible(0, trace.pos)
    {
        return false;
    }
    let expiry = trace
        .dry_tick
        .map(|dry| dry.saturating_add(trace.kind.silt_ticks()))
        .or_else(|| {
            trace.wash_start.map(|start| {
                start
                    .saturating_add(trace.kind.wash_ticks())
                    .saturating_add(trace.kind.silt_ticks())
            })
        });
    expiry.is_none_or(|until| tick < until)
}

/// Convert a logical screen pixel back to a conservative map cell.  The
/// inverse is intentionally approximate: a sprite pixel is allowed only when
/// its nearest cell is still the trace's lane and visible to player 0.
fn projected_cell(camera: Camera, sx: i32, sy: i32) -> (i32, i32) {
    let u = f64::from(sx + camera.x) / 16.0;
    let v = f64::from(sy + camera.y) / 8.0;
    let raw_x = (u + v) * f64::from(FP) / 2.0;
    let raw_y = (v - u) * f64::from(FP) / 2.0;
    (
        (raw_x / f64::from(FP)).floor() as i32,
        (raw_y / f64::from(FP)).floor() as i32,
    )
}

fn pixel_is_allowed(
    world: &World,
    camera: Camera,
    trace: &Trace,
    sx: i32,
    sy: i32,
    visible_cells: &mut BTreeMap<(i32, i32), bool>,
) -> bool {
    let (cell_x, cell_y) = projected_cell(camera, sx, sy);
    if world.map.terrain(cell_x, cell_y) != trace.lane {
        return false;
    }
    if let Some(visible) = visible_cells.get(&(cell_x, cell_y)) {
        return *visible;
    }
    if visible_cells.len() >= MAX_VISIBILITY_CELLS {
        visible_cells.clear();
    }
    let visible = world.visible(0, Pos::cell(cell_x, cell_y));
    visible_cells.insert((cell_x, cell_y), visible);
    visible
}

/// Fallback for an atlas that predates the v10 trace contract.  It is a small
/// deterministic diagnostic mark, never an entity-like prop; authored phase
/// sprites take precedence as soon as their keys are present.
fn fallback_pixel(kind: TraceKind, phase: u8, xx: i32, yy: i32) -> Option<[u8; 4]> {
    let mut color = match kind {
        TraceKind::JackFoot => [211, 173, 99, 220],
        TraceKind::Rivet => [187, 198, 174, 210],
        TraceKind::Binding => [164, 205, 170, 205],
        TraceKind::Glaze => [164, 173, 224, 210],
        TraceKind::Invalid => return None,
    };
    let fade = match phase {
        0 => 100,
        1 => 90,
        2 => 78,
        3 => 65,
        4 => 52,
        _ => 35,
    };
    for channel in color.iter_mut().take(3) {
        *channel = (*channel as u16 * fade / 100) as u8;
    }
    color[3] = ((u16::from(color[3]) * fade / 100) as u8).max(1);
    let hit = match kind {
        TraceKind::JackFoot => {
            ((8..12).contains(&xx) && (6..8).contains(&yy))
                || ((20..24).contains(&xx) && (6..8).contains(&yy))
                || ((10..12).contains(&xx) && (3..6).contains(&yy))
                || ((22..24).contains(&xx) && (3..6).contains(&yy))
        }
        TraceKind::Rivet => {
            (xx, yy) == (11, 7) || (xx, yy) == (14, 5) || (xx, yy) == (18, 5) || (xx, yy) == (21, 7)
        }
        TraceKind::Binding => {
            ((10..22).contains(&xx) && yy == (xx - 10) / 3)
                || ((11..21).contains(&xx) && yy == 4 - ((xx - 10) / 3))
                || (xx, yy) == (23, 7)
        }
        TraceKind::Glaze => {
            ((15..18).contains(&xx) && yy == 6) || ((14..19).contains(&xx) && yy == 7)
        }
        TraceKind::Invalid => false,
    };
    hit.then_some(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{Faction, Kind};

    fn event(tick: u64, kind: EventKind) -> Event {
        Event {
            tick,
            kind,
            player: None,
            entity: None,
            other: None,
            from: None,
            to: None,
            amount: 0,
            text: String::new(),
            cause: None,
        }
    }

    fn make_world() -> World {
        let mut world = World::new(7, Faction::Union);
        world.ai_enabled = false;
        world
    }

    fn make_visible(world: &mut World, pos: Pos) -> u32 {
        let entity = world
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Hook)
            .expect("starting worker");
        entity.pos = pos;
        entity.id
    }

    #[test]
    fn hidden_enemy_shot_and_death_do_not_change_marks() {
        let mut world = make_world();
        world.tick = 12;
        let hidden = Pos::cell(64, 49);
        let mut traces = TraceState::default();
        let mut shot = event(11, EventKind::Shot);
        shot.player = Some(1);
        shot.from = Some(hidden);
        shot.to = Some(hidden);
        traces.record_event(&world, &shot);
        let mut death = event(11, EventKind::Death);
        death.player = Some(1);
        death.from = Some(hidden);
        traces.record_event(&world, &death);
        assert!(traces.traces.is_empty());
    }

    #[test]
    fn visible_union_shot_uses_event_position_and_rejects_nonlane() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        make_visible(&mut world, lane);
        world.tick = 4;
        let mut traces = TraceState::default();
        let mut shot = event(3, EventKind::Shot);
        shot.player = Some(0);
        shot.from = Some(lane);
        shot.to = Some(lane);
        traces.record_event(&world, &shot);
        assert_eq!(traces.traces.len(), 1);
        let mut nonlane = event(4, EventKind::Shot);
        nonlane.player = Some(0);
        nonlane.from = Some(Pos::cell(64, 64));
        nonlane.to = nonlane.from;
        traces.record_event(&world, &nonlane);
        assert_eq!(traces.traces.len(), 1);
    }

    #[test]
    fn visible_compact_shot_fuses_glaze_only_at_its_impact() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        make_visible(&mut world, lane);
        world.players[0].faction = Faction::Compact;
        world.tick = 4;
        let mut traces = TraceState::default();
        let mut shot = event(3, EventKind::Shot);
        shot.player = Some(0);
        shot.from = Some(lane);
        shot.to = Some(lane);
        traces.record_event(&world, &shot);
        assert_eq!(traces.traces.len(), 1);
        assert_eq!(traces.traces[0].kind, TraceKind::Glaze);
        assert_eq!(traces.traces[0].kind.asset_name(), "glaze");
        // No impact point, no glass: the firing point is never glazed.
        let mut blind = event(4, EventKind::Shot);
        blind.player = Some(0);
        blind.from = Some(lane);
        traces.record_event(&world, &blind);
        assert_eq!(traces.traces.len(), 1);
        // The sidecar keeps the new kind through a save.
        let saved = serde_json::to_string(&traces).unwrap();
        let loaded: TraceState = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded, traces);
        assert!(fallback_pixel(TraceKind::Glaze, 0, 16, 7).is_some());
    }

    #[test]
    fn deployment_observer_requires_visible_prior_state() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        let id = make_visible(&mut world, lane);
        let entity = world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .unwrap();
        entity.kind = Kind::Bulwark;
        let mut traces = TraceState::default();
        traces.observe_deployments(&world);
        world.tick = 1;
        world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .unwrap()
            .deployed = true;
        traces.observe_deployments(&world);
        assert_eq!(traces.traces.len(), 1);

        let mut hidden_world = make_world();
        let hidden_id = hidden_world
            .entities
            .iter()
            .find(|entity| entity.owner == 1 && entity.kind.is_worker())
            .map(|entity| entity.id)
            .unwrap();
        let mut hidden = TraceState::default();
        hidden_world
            .entities
            .iter_mut()
            .find(|entity| entity.id == hidden_id)
            .unwrap()
            .kind = Kind::Bulwark;
        hidden.observe_deployments(&hidden_world);
        hidden_world
            .entities
            .iter_mut()
            .find(|entity| entity.id == hidden_id)
            .unwrap()
            .deployed = true;
        hidden.observe_deployments(&hidden_world);
        assert!(hidden.traces.is_empty());
    }

    #[test]
    fn deployment_toggle_while_hidden_is_not_inferred_on_reentry() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        make_visible(&mut world, lane);
        let hidden_pos = Pos::cell(64, 30);
        let enemy_id = world
            .entities
            .iter_mut()
            .find(|entity| entity.owner == 1 && entity.kind.is_worker())
            .map(|entity| {
                entity.kind = Kind::Bulwark;
                entity.pos = lane;
                entity.id
            })
            .unwrap();
        let mut traces = TraceState::default();
        world.tick = 1;
        traces.observe_deployments(&world);
        world.tick = 2;
        world
            .entities
            .iter_mut()
            .find(|entity| entity.id == enemy_id)
            .unwrap()
            .pos = hidden_pos;
        traces.observe_deployments(&world);
        world.tick = 3;
        let enemy = world
            .entities
            .iter_mut()
            .find(|entity| entity.id == enemy_id)
            .unwrap();
        enemy.deployed = true;
        traces.observe_deployments(&world);
        world.tick = 4;
        world
            .entities
            .iter_mut()
            .find(|entity| entity.id == enemy_id)
            .unwrap()
            .pos = lane;
        traces.observe_deployments(&world);
        assert!(traces.traces.is_empty());
    }

    #[test]
    fn gate_change_washes_only_newly_flooded_lane_and_dry_sets_silt() {
        let mut world = make_world();
        let north = Pos::cell(64, 49);
        make_visible(&mut world, north);
        world.tick = 10;
        let mut traces = TraceState::default();
        let mut shot = event(9, EventKind::Shot);
        shot.player = Some(0);
        shot.from = Some(north);
        shot.to = Some(north);
        traces.record_event(&world, &shot);
        traces.traces.push(Trace {
            kind: TraceKind::Rivet,
            pos: Pos::cell(64, 79),
            lane: Terrain::Lane1,
            born_tick: 10,
            wash_start: None,
            dry_tick: None,
        });
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(false);
        world.tick = 11;
        traces.record_event(&world, &event(10, EventKind::GateChanged));
        assert_eq!(
            traces
                .traces
                .iter()
                .filter(|trace| trace.wash_start.is_some())
                .count(),
            1
        );
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm::from_north(true);
        world.tick = 12;
        traces.record_event(&world, &event(11, EventKind::GateChanged));
        assert!(
            traces
                .traces
                .iter()
                .find(|trace| trace.lane == Terrain::Lane0)
                .is_some_and(|trace| trace.dry_tick.is_some())
        );
        assert!(
            traces
                .traces
                .iter()
                .find(|trace| trace.lane == Terrain::Lane1)
                .is_some_and(|trace| trace.wash_start.is_some())
        );
    }

    #[test]
    fn a_third_arm_lane_keeps_and_washes_its_marks_on_its_own_tide() {
        let mut world = World::with_map(
            7,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .unwrap();
        world.ai_enabled = false;
        world.tick = 10;
        let mark = |x: i32, y: i32, lane: Terrain| Trace {
            kind: TraceKind::Rivet,
            pos: Pos::cell(x, y),
            lane,
            born_tick: 10,
            wash_start: Some(5),
            dry_tick: None,
        };
        let centres: Vec<(i32, i32)> = world.map.layout().arms.iter().map(|a| a.centre).collect();
        let mut traces = TraceState::default();
        for (arm, &(x, y)) in centres.iter().enumerate() {
            traces.traces.push(mark(x, y, Terrain::lane(arm as u8)));
        }
        traces.sanitize(&world);
        assert_eq!(traces.traces.len(), 3, "a Lane2 mark survives sanitize");
        // S (arm 2) runs dry: only its mark settles to silt; E and W flood
        // and keep washing.
        world.gate.tide = bw_sim::Tide::Open;
        world.gate.dry_arm = bw_sim::Arm(2);
        world.tick = 11;
        traces.record_event(&world, &event(11, EventKind::GateChanged));
        for trace in &traces.traces {
            let dry = trace.lane == Terrain::Lane2;
            assert_eq!(trace.dry_tick.is_some(), dry, "{:?}", trace.lane);
        }
        // W runs dry next: S floods and washes again, W settles.
        world.gate.dry_arm = bw_sim::Arm(1);
        world.tick = 12;
        traces.record_event(&world, &event(12, EventKind::GateChanged));
        for trace in &traces.traces {
            let dry = trace.lane == Terrain::Lane1;
            assert_eq!(trace.dry_tick.is_some(), dry, "{:?}", trace.lane);
        }
        let s = traces
            .traces
            .iter()
            .find(|t| t.lane == Terrain::Lane2)
            .unwrap();
        assert_eq!(s.wash_start, Some(12));
    }

    #[test]
    fn sidecar_roundtrip_sanitize_and_cap_are_bounded() {
        let mut world = make_world();
        world.tick = 20;
        let mut traces = TraceState::default();
        for i in 0..(MAX_TRACES + 10) {
            traces.traces.push(Trace {
                kind: TraceKind::Rivet,
                pos: Pos::cell(52 + (i as i32 % 24), 49),
                lane: Terrain::Lane0,
                born_tick: i as u64,
                wash_start: None,
                dry_tick: None,
            });
        }
        traces.traces.push(Trace {
            kind: TraceKind::Binding,
            pos: Pos::cell(1, 1),
            lane: Terrain::Lane0,
            born_tick: 2,
            wash_start: None,
            dry_tick: None,
        });
        traces.traces.push(Trace {
            kind: TraceKind::Binding,
            pos: Pos::cell(64, 49),
            lane: Terrain::Lane0,
            born_tick: 99,
            wash_start: None,
            dry_tick: None,
        });
        traces.sanitize(&world);
        assert!(traces.traces.len() <= MAX_TRACES);
        assert!(
            traces
                .traces
                .iter()
                .all(|trace| trace.born_tick <= world.tick)
        );
        assert!(
            traces
                .traces
                .iter()
                .all(|trace| lane_at(&world, trace.pos) == Some(trace.lane))
        );
        let bytes = serde_json::to_vec(&traces).unwrap();
        let roundtrip: TraceState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(traces, roundtrip);
    }

    #[test]
    fn scheduled_wash_survives_sanitize_and_is_fresh_before_start() {
        let mut world = make_world();
        world.tick = 10;
        let mut traces = TraceState {
            traces: vec![Trace {
                kind: TraceKind::Rivet,
                pos: Pos::cell(64, 49),
                lane: Terrain::Lane0,
                born_tick: 10,
                wash_start: Some(28),
                dry_tick: None,
            }],
            observed_deployed: BTreeMap::new(),
        };
        traces.sanitize(&world);
        assert_eq!(traces.traces[0].wash_start, Some(28));
        assert_eq!(trace_phase(&traces.traces[0], 27), 0);
        assert_eq!(trace_phase(&traces.traces[0], 28), 1);

        traces.traces.push(Trace {
            kind: TraceKind::Binding,
            pos: Pos::cell(64, 49),
            lane: Terrain::Lane0,
            born_tick: 10,
            wash_start: Some(40),
            dry_tick: None,
        });
        traces.sanitize(&world);
        assert_eq!(traces.traces[1].wash_start, None);
    }

    #[test]
    fn fallback_draw_is_deterministic_and_never_ticks_state() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        make_visible(&mut world, lane);
        world.tick = 3;
        let mut traces = TraceState::default();
        let mut shot = event(2, EventKind::Shot);
        shot.player = Some(0);
        shot.from = Some(lane);
        traces.record_event(&world, &shot);
        let before = traces.clone();
        let mut first = Canvas::default();
        let mut second = Canvas::default();
        let mut camera = Camera::default();
        camera.center(lane);
        assert_eq!(traces.draw(&mut first, None, &world, camera, 3), 1);
        assert_eq!(traces.draw(&mut second, None, &world, camera, 3), 1);
        assert_eq!(first.pixels, second.pixels);
        assert_eq!(traces, before);
    }

    #[test]
    fn fallback_trace_draw_uses_the_wide_surface_clip() {
        let mut world = make_world();
        let lane = Pos::cell(64, 49);
        make_visible(&mut world, lane);
        world.tick = 3;
        let mut traces = TraceState::default();
        let mut shot = event(2, EventKind::Shot);
        shot.player = Some(0);
        shot.from = Some(lane);
        traces.record_event(&world, &shot);

        let mut camera = Camera::default();
        camera.center(lane);
        camera.x -= 520;
        camera.y -= 300;
        let mut canvas = Canvas::new(1280, 552);
        assert_eq!(traces.draw(&mut canvas, None, &world, camera, 3), 1);
        let wide_pixels = (640..canvas.width()).any(|x| {
            (360..canvas.height()).any(|y| {
                let index = ((y * canvas.width() + x) * 4) as usize;
                canvas.pixels[index + 3] != 0
            })
        });
        assert!(wide_pixels, "trace rendering retained the 640x360 clip");
    }
}
