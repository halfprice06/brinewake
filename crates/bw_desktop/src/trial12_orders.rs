//! Orders and controls from the twelfth trial (rules 21, three seats on the
//! Confluence): attack, attack-move, gather and capture orders nobody
//! could carry out are refused at once and name the deep lane that
//! blocks them; E boards (B stays BUILD); a transport shows its riders; a
//! new order says when it drops a running capture, and a queued one keeps
//! it; a group order that packs a deployed gun says K keeps it; a group
//! key's recentre waits for the rest of a batch of input; a box at the
//! field's lower edge takes machines whose feet are under the dock; and a
//! worker given a site a moment ago is not idle.
use crate::game::Game;
use crate::hover_card::{Chip, Mark};
use bw_core::{FP, Kind, Pos};
use bw_sim::{Command, Event, EventKind, Order, World};

/// A worker beside a wreck: one cell, diagonals too.
const BESIDE_WRECK: i32 = FP * 3 / 2;
/// The station's capture ring, as the simulation draws it.
const CAPTURE_RING: i32 = FP * 2;
/// How far a machine's body rises over its feet on the field, in scene
/// pixels at the base zoom: a box over the body takes it.
const BODY_PIXELS: f64 = 20.0;
/// The prompt after a group order packs a deployed gun (trial 12: players
/// learned K from the prompt only after the guns had packed).
pub(crate) const PACKED_NOTE: &str = " PACKED - K KEEPS DEPLOYED.";
/// The prompt when a new order ends a running capture claim.
pub(crate) const CAPTURE_DROPPED: &str = " CAPTURE DROPPED.";
/// The prompt when capturers are left out of a queued order.
pub(crate) const CAPTURE_KEPT: &str = " CAPTURE KEPT.";

/// Input state of the twelfth trial's controls.
#[derive(Clone, Debug, Default)]
pub(crate) struct OrdersState {
    /// A batch of input is being applied (the play harness sends several
    /// keys and clicks read off one picture).
    pub(crate) input_batch: bool,
    /// A recentre asked for during the batch, applied after it.
    pub(crate) pending_recentre: Option<Pos>,
}

impl Game {
    /// Why no selected machine could carry out `command`, or None when one
    /// can.  Move and attack-move need the point; an attack needs a cell
    /// within the weapon's reach of the target; a gather a cell beside the
    /// wreck; a capture a cell in the station's ring.  Rallies are never
    /// refused: they outlast the tide.
    pub(crate) fn unreachable_order(&self, command: &Command) -> Option<String> {
        match command {
            Command::Move { units, target, .. } | Command::AttackMove { units, target, .. } => {
                self.no_way_there(units, *target)
            }
            Command::Attack { units, target } => {
                let aim = self
                    .world
                    .entities
                    .iter()
                    .find(|e| e.id == *target)
                    .map(|e| e.pos)?;
                self.refuse_unless(units, |w| {
                    w.any_can_reach_within(units, aim, |e| w.weapon_range(e))
                })
            }
            Command::Gather { units, resource } | Command::QueueGather { units, resource } => {
                let wreck = self
                    .world
                    .map
                    .resources
                    .iter()
                    .find(|r| r.id == *resource)
                    .map(|r| r.pos)?;
                self.refuse_unless(units, |w| {
                    w.any_can_reach_within(units, wreck, |_| BESIDE_WRECK)
                })
            }
            Command::Capture { units } => {
                let gate = self.world.map.gate_pos;
                self.refuse_unless(units, |w| {
                    w.any_can_reach_within(units, gate, |_| CAPTURE_RING)
                })
            }
            _ => None,
        }
    }

    fn refuse_unless(&self, units: &[u32], reachable: impl Fn(&World) -> bool) -> Option<String> {
        if units.is_empty() || reachable(&self.world) {
            return None;
        }
        Some(self.no_way_words(reachable))
    }

    /// Why an order cannot be carried out, naming the deep lane on its way:
    /// the lanes any one of which, dried, would let it through; or every
    /// deep lane when only all of them together would.  A lane that stands
    /// dry or shallow is never named (trial 12: "the lane is deep" came
    /// whenever any lane was, beside a card calling the clicked lane DRY).
    pub(crate) fn no_way_words(&self, reachable: impl Fn(&World) -> bool) -> String {
        let arms = crate::seats::arm_count(&self.world);
        let closed: Vec<usize> = (0..arms)
            .filter(|arm| crate::lane_wall::lane_closed(&self.world, *arm))
            .collect();
        let dried = |open: &[usize]| {
            let mut world = self.world.clone();
            dry_lanes(&mut world, open);
            reachable(&world)
        };
        let mut blocking: Vec<usize> = closed
            .iter()
            .copied()
            .filter(|arm| dried(&[*arm]))
            .collect();
        if blocking.is_empty() && closed.len() > 1 && dried(&closed) {
            blocking = closed;
        }
        let letters: Vec<&str> = blocking
            .iter()
            .map(|arm| crate::seats::arm_letter(&self.world, *arm))
            .collect();
        match letters.as_slice() {
            [] => "No way there from here.".into(),
            [one] => format!("No way across: the {one} lane is deep."),
            [first @ .., last] => format!(
                "No way across: the {} and {last} lanes are deep.",
                first.join(", ")
            ),
        }
    }

    /// Own machines in `units` whose capture claim runs: walking to the
    /// station or channelling there, or packing to go.
    fn capturers(&self, units: &[u32]) -> Vec<u32> {
        self.world
            .entities
            .iter()
            .filter(|e| {
                units.contains(&e.id)
                    && e.owner == 0
                    && e.hp > 0
                    && (matches!(e.order, Order::Capture)
                        || matches!(e.after_pack, Some(Order::Capture)))
            })
            .map(|e| e.id)
            .collect()
    }

    /// A queued move or attack-move for machines on a capture claim
    /// leaves them out: the simulation's waypoints would replace the claim
    /// when a walking capturer arrives, and a channelling one never walks
    /// them.  The claim goes on and the prompt says so.  None when every
    /// machine of the order is capturing.
    pub(crate) fn keep_capture_on_queue(&self, command: Command) -> Option<(Command, bool)> {
        let (units, target, queued, attack) = match &command {
            Command::Move {
                units,
                target,
                queued,
            } => (units, *target, *queued, false),
            Command::AttackMove {
                units,
                target,
                queued,
            } => (units, *target, *queued, true),
            _ => return Some((command, false)),
        };
        if !queued {
            return Some((command, false));
        }
        let capturers = self.capturers(units);
        if capturers.is_empty() {
            return Some((command, false));
        }
        let units: Vec<u32> = units
            .iter()
            .copied()
            .filter(|id| !capturers.contains(id))
            .collect();
        if units.is_empty() {
            return None;
        }
        let command = if attack {
            Command::AttackMove {
                units,
                target,
                queued,
            }
        } else {
            Command::Move {
                units,
                target,
                queued,
            }
        };
        Some((command, true))
    }

    /// Whether `command` replaces a running capture claim of one of its
    /// machines (trial 12: an attack order dropped a claim at 60%).
    pub(crate) fn drops_capture(&self, command: &Command) -> bool {
        let units = match command {
            Command::Move {
                units,
                queued: false,
                ..
            }
            | Command::AttackMove {
                units,
                queued: false,
                ..
            }
            | Command::Attack { units, .. }
            | Command::Gather { units, .. }
            | Command::Stop { units }
            | Command::Hold { units }
            | Command::Board { units, .. }
            | Command::Deploy { units }
            | Command::SetDeployed { units, .. }
            | Command::Face { units, .. } => units,
            _ => return false,
        };
        !self.capturers(units).is_empty()
    }

    /// Recentre the view on `pos`, or after the batch of input being
    /// applied: a click read off the same picture as the group key must
    /// land where the picture showed (trial 12: a double-tapped group key
    /// moved the view under the next click).
    pub(crate) fn recentre(&mut self, pos: Pos) {
        if self.orders12.input_batch {
            self.orders12.pending_recentre = Some(pos);
        } else {
            self.camera.center(pos);
            self.camera_sub = (0.0, 0.0);
        }
    }

    /// A batch of input read off one picture begins.
    pub(crate) fn begin_input_batch(&mut self) {
        self.orders12.input_batch = true;
    }

    /// The batch is applied: a recentre it asked for happens now.
    pub(crate) fn end_input_batch(&mut self) {
        self.orders12.input_batch = false;
        if let Some(pos) = self.orders12.pending_recentre.take() {
            self.recentre(pos);
        }
    }

    /// Whether a box from (x0, y0) to (x1, y1) takes the machine standing
    /// at `pos`: its feet or its body in the box, and some of it on the
    /// field.  Only the feet were tested, so a machine near the dock whose
    /// feet stood under it could not be boxed (trial 12).
    pub(crate) fn box_takes(&self, pos: Pos, (x0, y0): (i32, i32), (x1, y1): (i32, i32)) -> bool {
        let view = self.world_view();
        let (px, py) = self.project(pos);
        let body = (BODY_PIXELS * view.zoom.scale()).round() as i32;
        let (top, bottom) = (py - body, py);
        let on_field = (0..view.width).contains(&px) && top < view.bottom && bottom >= view.top;
        on_field
            && px >= x0.min(x1)
            && px <= x0.max(x1)
            && top <= y0.max(y1)
            && bottom >= y0.min(y1)
    }

    /// Whether a worker's site order may still be on its way: a site given
    /// in the last three seconds, or sites saved for later.  Such a worker
    /// is not idle, whatever its order says this tick (trial 12: Shift+I
    /// took a worker just given a site, and the site stood without one).
    pub(crate) fn worker_order_pending(&self, worker: &bw_sim::Entity) -> bool {
        let tick = self.world.tick;
        !worker.build_queue.is_empty()
            || self
                .recent_builders
                .iter()
                .any(|(id, at)| *id == worker.id && tick.saturating_sub(*at) < 90)
    }

    /// The riders of a selected transport as a mark and a number, for the
    /// selection panel.
    pub(crate) fn rider_chip(&self, entity: &bw_sim::Entity) -> Option<Chip> {
        entity.kind.is_transport().then(|| Chip {
            mark: Mark::Crew,
            value: format!("{}/{}", entity.cargo.len(), bw_content::TRANSPORT_CAPACITY),
            short: false,
        })
    }

    /// A loaded own transport on the field carries its rider count, so
    /// machines aboard are not lost from sight (trial 12: nine Sounders
    /// boarded by accident and were never seen again).
    pub(crate) fn draw_cargo_pips(&mut self) {
        let loaded: Vec<(Pos, usize)> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.aboard.is_none()
                    && e.kind.is_transport()
                    && !e.cargo.is_empty()
            })
            .map(|e| (e.pos, e.cargo.len()))
            .collect();
        for (pos, riders) in loaded {
            let (x, y) = self.project(pos);
            crate::field_labels::draw_badge(
                &mut self.canvas,
                &riders.to_string(),
                x + 14,
                y - 30,
                crate::canvas::GOLD,
            );
        }
    }
}

/// Dry every lane of `arms` in a copy of the world, as a Salter's crust
/// would: the question is only whether the way opens.
fn dry_lanes(world: &mut World, arms: &[usize]) {
    let width = i32::from(world.map.width);
    for y in 0..i32::from(world.map.height) {
        for x in 0..width {
            if world
                .map
                .terrain(x, y)
                .tidal_arm()
                .is_some_and(|arm| arms.contains(&usize::from(arm)))
            {
                world.crust.push((y * width + x) as u32);
            }
        }
    }
    world.crust.sort_unstable();
    world.crust.dedup();
}

/// "LIFTER LOST: 9 ABOARD." when an own transport sank with machines in
/// its hold this tick: they die with it and were never on the field.  A
/// transport has no gun, so a death whose cause is one is a rider that
/// sank with it (the simulation names the transport as `other`).
pub(crate) fn riders_lost(events: &[Event]) -> Option<String> {
    let mut sunk: Vec<(u32, Kind, usize)> = Vec::new();
    for rider in events
        .iter()
        .filter(|e| e.kind == EventKind::Death && e.player == Some(0))
    {
        let (Some(transport), Some(kind)) = (rider.other, rider.cause) else {
            continue;
        };
        if !kind.is_transport() {
            continue;
        }
        match sunk.iter_mut().find(|(id, _, _)| *id == transport) {
            Some(entry) => entry.2 += 1,
            None => sunk.push((transport, kind, 1)),
        }
    }
    let riders: usize = sunk.iter().map(|(_, _, n)| n).sum();
    let (_, kind, _) = sunk.first()?;
    Some(if sunk.len() == 1 {
        format!("{} LOST: {riders} ABOARD.", kind.name().to_uppercase())
    } else {
        format!("{} TRANSPORTS LOST: {riders} ABOARD.", sunk.len())
    })
}

#[cfg(test)]
mod tests {
    use crate::game::{Action, Game, Mode};
    use bw_core::{Faction, Kind, Pos, Terrain};
    use bw_sim::{Arm, Command, MapId, Order, Tide};
    use std::path::PathBuf;

    fn game_on(map: MapId, faction: Faction) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = std::env::temp_dir().join(format!(
            "bw-trial12-orders-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut game = Game::new_with_data_dir(base, data);
        game.faction = faction;
        game.map = map;
        game.start();
        game.world.ai_enabled = false;
        game.resize_view(1280, 720);
        game.selected.clear();
        game.render();
        game
    }

    fn confluence() -> Game {
        let g = game_on(MapId::Confluence, Faction::Union);
        assert_eq!(g.world.seat_count(), 3);
        g
    }

    fn hq(g: &Game, seat: u8) -> (u32, Pos) {
        g.world
            .entities
            .iter()
            .find(|e| e.owner == seat && e.kind == Kind::Headquarters)
            .map(|e| (e.id, e.pos))
            .expect("headquarters")
    }

    /// Dry open ground near `seat`'s headquarters, `dx`, `dy` cells off.
    fn ground(g: &Game, seat: u8, dx: i32, dy: i32) -> Pos {
        let (x, y) = hq(g, seat).1.cell_xy();
        for r in 0..10 {
            for oy in -r..=r {
                for ox in -r..=r {
                    let (cx, cy) = (x + dx + ox, y + dy + oy);
                    let pos = Pos::cell(cx, cy);
                    if !g.world.entities.iter().any(|e| {
                        let (ex, ey) = e.pos.cell_xy();
                        let n = bw_content::spec(e.kind).footprint.max(1);
                        e.hp > 0 && (ex..ex + n).contains(&cx) && (ey..ey + n).contains(&cy)
                    }) && matches!(g.world.map.terrain(cx, cy), Terrain::Salt | Terrain::Silt)
                    {
                        return pos;
                    }
                }
            }
        }
        panic!("no open ground near seat {seat}");
    }

    fn unit(g: &mut Game, seat: u8, kind: Kind, at: Pos) -> u32 {
        let id = g.world.spawn_for_tests(seat, kind, at);
        if let Some(e) = g.world.entities.iter_mut().find(|e| e.id == id) {
            e.order = Order::Idle;
        }
        id
    }

    /// A machine of `seat` on open ground near `near`'s headquarters.
    fn put(g: &mut Game, seat: u8, kind: Kind, near: u8, dx: i32, dy: i32) -> u32 {
        let at = ground(g, near, dx, dy);
        unit(g, seat, kind, at)
    }

    /// The arm whose lane joins two seats' banks.
    fn arm_between(g: &Game, a: u8, b: u8) -> usize {
        (0..crate::seats::arm_count(&g.world))
            .find(|arm| {
                let banks = g.world.arm_banks(*arm);
                banks.contains(&a) && banks.contains(&b)
            })
            .expect("a lane between the two seats")
    }

    fn letter(g: &Game, arm: usize) -> &'static str {
        crate::seats::arm_letter(&g.world, arm)
    }

    fn open_tide(g: &mut Game, dry: usize) {
        g.world.gate.tide = Tide::Open;
        g.world.gate.dry_arm = Arm(dry as u8);
    }

    /// Issue through the interface and say whether it reached the log.
    fn sent(g: &mut Game, command: Command) -> bool {
        let before = g.world.command_log.len();
        g.message.clear();
        let accepted = g.issue_accepted(command);
        if !accepted {
            assert_eq!(
                g.world.command_log.len(),
                before,
                "a refused order is not sent"
            );
        }
        accepted
    }

    #[test]
    fn an_attack_across_a_deep_lane_is_refused_and_names_that_lane() {
        let mut g = confluence();
        g.world.gate.tide = Tide::Flood;
        let own = put(&mut g, 0, Kind::Riveter, 0, 5, 5);
        let (target, _) = hq(&g, 1);
        let attack = Command::Attack {
            units: vec![own],
            target,
        };
        assert!(!sent(&mut g, attack.clone()));
        let lane = letter(&g, arm_between(&g, 0, 1));
        assert_eq!(
            g.message,
            format!("No way across: the {lane} lane is deep."),
            "only the lane on the way is named"
        );
        // The tide falls: the way is open (the headquarters is in the fog,
        // which the simulation judges, not this check).
        g.world.gate.tide = Tide::Neutral;
        assert_eq!(g.unreachable_order(&attack), None);
    }

    #[test]
    fn an_attack_on_a_target_in_reach_is_sent_whatever_the_tide() {
        let mut g = confluence();
        g.world.gate.tide = Tide::Flood;
        let at = ground(&g, 0, 5, 5);
        let own = unit(&mut g, 0, Kind::Riveter, at);
        let (x, y) = at.cell_xy();
        let foe = unit(&mut g, 1, Kind::Riveter, Pos::cell(x + 1, y));
        assert!(sent(
            &mut g,
            Command::Attack {
                units: vec![own],
                target: foe,
            }
        ));
    }

    #[test]
    fn an_attack_move_names_only_the_deep_lanes_that_block_it() {
        let mut g = confluence();
        let own = put(&mut g, 0, Kind::Riveter, 0, 5, 5);
        let to_one = arm_between(&g, 0, 1);
        let to_two = arm_between(&g, 0, 2);
        let far = arm_between(&g, 1, 2);
        // The lane to seat 1 dry, the others deep: seat 1 is reachable,
        // seat 2 is not, and the dry lane is never called deep (trial 12,
        // 33:43: "the lane is deep" beside a card saying E DRY).
        open_tide(&mut g, to_one);
        let one = hq(&g, 1).1;
        let two = hq(&g, 2).1;
        assert!(sent(
            &mut g,
            Command::AttackMove {
                units: vec![own],
                target: one,
                queued: false,
            }
        ));
        assert!(!sent(
            &mut g,
            Command::AttackMove {
                units: vec![own],
                target: two,
                queued: false,
            }
        ));
        let (a, b) = if to_two < far {
            (to_two, far)
        } else {
            (far, to_two)
        };
        assert_eq!(
            g.message,
            format!(
                "No way across: the {} and {} lanes are deep.",
                letter(&g, a),
                letter(&g, b)
            )
        );
        // A-click on the ground goes the same way through the interface.
        g.selected = vec![own];
        g.mode = Mode::Attack;
        let before = g.world.command_log.len();
        g.attack_destination(two);
        assert_eq!(g.world.command_log.len(), before);
        assert!(g.message.contains("lanes are deep"), "{}", g.message);
        assert_eq!(g.mode, Mode::Attack, "another point may be picked");
    }

    #[test]
    fn the_split_basin_names_its_lanes_by_letter() {
        let mut g = game_on(MapId::SplitBasin, Faction::Union);
        let own = put(&mut g, 0, Kind::Riveter, 0, 5, 5);
        let enemy = hq(&g, 1).1;
        let go = Command::Move {
            units: vec![own],
            target: enemy,
            queued: false,
        };
        g.world.gate.tide = Tide::Flood;
        assert!(!sent(&mut g, go.clone()));
        assert_eq!(g.message, "No way across: the N and S lanes are deep.");
        // One lane dry: the way is open, so nothing is refused.
        open_tide(&mut g, 0);
        assert!(sent(&mut g, go));
    }

    #[test]
    fn a_gather_on_a_wreck_across_a_deep_lane_is_refused_at_once() {
        let mut g = confluence();
        g.world.gate.tide = Tide::Flood;
        let worker = put(&mut g, 0, Faction::Union.worker(), 0, 4, 4);
        let nearest_wreck = |g: &Game, to: Pos| {
            g.world
                .map
                .resources
                .iter()
                .filter(|r| r.remaining > 0)
                .min_by_key(|r| r.pos.distance_sq(to))
                .map(|r| r.id)
                .expect("a wreck")
        };
        let far = nearest_wreck(&g, hq(&g, 1).1);
        assert!(!sent(
            &mut g,
            Command::Gather {
                units: vec![worker],
                resource: far,
            }
        ));
        let lane = letter(&g, arm_between(&g, 0, 1));
        assert_eq!(
            g.message,
            format!("No way across: the {lane} lane is deep.")
        );
        assert!(!sent(
            &mut g,
            Command::QueueGather {
                units: vec![worker],
                resource: far,
            }
        ));
        // A home wreck still gathers.
        let near = nearest_wreck(&g, hq(&g, 0).1);
        assert!(sent(
            &mut g,
            Command::Gather {
                units: vec![worker],
                resource: near,
            }
        ));
    }

    #[test]
    fn a_capture_nobody_can_reach_is_refused_and_one_that_can_is_sent() {
        let mut g = confluence();
        let own = put(&mut g, 0, Kind::Riveter, 0, 5, 5);
        g.world.gate.tide = Tide::Flood;
        assert!(
            !g.world
                .any_can_reach_within(&[own], g.world.map.gate_pos, |_| super::CAPTURE_RING),
            "a flood cuts every bank off the station"
        );
        g.selected = vec![own];
        g.message.clear();
        let before = g.world.command_log.len();
        g.key("G", false, false);
        assert_eq!(g.world.command_log.len(), before, "{}", g.message);
        assert!(
            g.message.starts_with("No way across: the "),
            "{}",
            g.message
        );
        assert!(g.message.contains("deep"), "{}", g.message);
        g.world.gate.tide = Tide::Neutral;
        g.key("G", false, false);
        assert_eq!(g.world.command_log.len(), before + 1, "{}", g.message);
        assert!(g.message.starts_with("Capture order sent"), "{}", g.message);
    }

    #[test]
    fn e_boards_and_b_never_does_for_any_side() {
        for faction in [Faction::Union, Faction::Assembly, Faction::Compact] {
            let mut g = game_on(MapId::SplitBasin, faction);
            let transport = match faction {
                Faction::Assembly => Kind::Barge,
                _ => Kind::Lifter,
            };
            let carrier = put(&mut g, 0, transport, 0, 6, 6);
            let gun = put(&mut g, 0, faction.army()[0], 0, 7, 6);
            g.selected = vec![gun];
            g.render();
            let board = g
                .buttons
                .iter()
                .find(|b| b.action == Action::Board)
                .unwrap_or_else(|| panic!("{faction:?}: the BOARD button"));
            assert!(board.hint.starts_with("E\n"), "{:?}", board.hint);
            let before = g.world.command_log.len();
            g.key("B", false, false);
            assert_eq!(g.world.command_log.len(), before, "{faction:?}: B boarded");
            assert!(!matches!(g.mode, Mode::Build(_)));
            assert_eq!(g.message, "B builds with workers. E boards.");
            g.key("E", false, false);
            assert_eq!(
                g.world.command_log.len(),
                before + 1,
                "{faction:?}: {}",
                g.message
            );
            assert!(matches!(
                g.world.command_log.last().map(|r| &r.command),
                Some(Command::Board { transport, .. }) if *transport == carrier
            ));
            // With a worker, B still builds.
            let worker = put(&mut g, 0, faction.worker(), 0, 4, 4);
            g.selected = vec![worker];
            g.key("B", false, false);
            assert_eq!(g.mode, Mode::Build(Kind::Works), "{faction:?}");
            g.key("Escape", false, false);
            // A Works keeps E for its third machine.
            let works = g
                .world
                .spawn_for_tests(0, Kind::Works, ground(&g, 0, -8, -8));
            g.selected = vec![works];
            assert_eq!(
                g.production_hotkey_kind("E"),
                faction.army().get(2).copied(),
                "{faction:?}"
            );
        }
    }

    #[test]
    fn a_transport_shows_its_riders_and_its_loss_counts_them() {
        let mut g = game_on(MapId::SplitBasin, Faction::Union);
        let carrier = put(&mut g, 0, Kind::Lifter, 0, 6, 6);
        let riders: Vec<u32> = (0..3)
            .map(|i| put(&mut g, 0, Kind::Riveter, 0, 7 + i, 6))
            .collect();
        for e in g
            .world
            .entities
            .iter_mut()
            .filter(|e| riders.contains(&e.id))
        {
            e.aboard = Some(carrier);
        }
        let entity = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.id == carrier)
            .unwrap();
        entity.cargo = riders.clone();
        let entity = entity.clone();
        let chip = g.rider_chip(&entity).expect("a transport's riders");
        assert_eq!(chip.value, "3/4");
        assert_eq!(chip.mark, crate::hover_card::Mark::Crew);
        g.selected = vec![carrier];
        let at = g
            .world
            .entities
            .iter()
            .find(|e| e.id == carrier)
            .unwrap()
            .pos;
        g.camera.center(at);
        g.render();
        // For a look by eye: BW_TRIAL12_FRAMES names a folder.
        if let Ok(dir) = std::env::var("BW_TRIAL12_FRAMES") {
            std::fs::create_dir_all(&dir).expect("frame folder");
            g.canvas
                .save(&PathBuf::from(dir).join("loaded-lifter.png"))
                .expect("frame");
        }
        // The transport goes down with its hold.
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == carrier)
            .unwrap()
            .hp = 0;
        g.tick();
        assert_eq!(
            super::riders_lost(&g.world.events).as_deref(),
            Some("LIFTER LOST: 3 ABOARD.")
        );
    }

    #[test]
    fn a_new_order_says_it_drops_a_capture_and_a_queued_one_keeps_it() {
        let mut g = confluence();
        let a = put(&mut g, 0, Kind::Riveter, 0, 5, 5);
        let b = put(&mut g, 0, Kind::Riveter, 0, 6, 5);
        let foe = put(&mut g, 1, Kind::Riveter, 0, 8, 5);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.id == a)
            .unwrap()
            .order = Order::Capture;
        // Shift-queued: the capturer is left to its claim.
        let elsewhere = ground(&g, 0, 3, 8);
        assert!(!sent(
            &mut g,
            Command::Move {
                units: vec![a],
                target: elsewhere,
                queued: true,
            }
        ));
        assert_eq!(g.message, "CAPTURE KEPT.");
        assert!(sent(
            &mut g,
            Command::Move {
                units: vec![a, b],
                target: elsewhere,
                queued: true,
            }
        ));
        assert!(g.message.ends_with("CAPTURE KEPT."), "{}", g.message);
        match &g.world.command_log.last().unwrap().command {
            Command::Move { units, .. } => assert_eq!(units, &vec![b]),
            other => panic!("{other:?}"),
        }
        g.tick();
        assert!(matches!(
            g.world.entities.iter().find(|e| e.id == a).unwrap().order,
            Order::Capture
        ));
        // An attack replaces it, and says so (trial 12, 14:23).
        assert!(sent(
            &mut g,
            Command::Attack {
                units: vec![a, b],
                target: foe,
            }
        ));
        assert!(g.message.starts_with("Attack order sent."), "{}", g.message);
        assert!(g.message.ends_with("CAPTURE DROPPED."), "{}", g.message);
        // Without a claim, no word of one.
        g.tick();
        assert!(sent(&mut g, Command::Stop { units: vec![a, b] }));
        assert!(!g.message.contains("CAPTURE"), "{}", g.message);
    }

    #[test]
    fn a_group_order_that_packs_a_deployed_gun_says_k_keeps_it() {
        let mut g = game_on(MapId::SplitBasin, Faction::Assembly);
        let loom = put(&mut g, 0, Kind::Loom, 0, 6, 6);
        let rifle = put(&mut g, 0, Faction::Assembly.army()[0], 0, 7, 6);
        g.issue(Command::Deploy { units: vec![loom] });
        for _ in 0..120 {
            g.tick();
        }
        assert!(
            g.world
                .entities
                .iter()
                .find(|e| e.id == loom)
                .unwrap()
                .deployed
        );
        g.issue(Command::Move {
            units: vec![loom, rifle],
            target: ground(&g, 0, 2, 9),
            queued: false,
        });
        assert!(
            g.message.ends_with("PACKED - K KEEPS DEPLOYED."),
            "{}",
            g.message
        );
        // The D card says it before: a group order packs, K keeps.
        g.selected = vec![loom];
        let words = crate::trial11_words::deploy_description(&g);
        assert!(words.contains("K keeps them deployed"), "{words}");
        assert!(words.contains("group order packs"), "{words}");
    }

    #[test]
    fn a_group_key_recentre_waits_for_the_rest_of_the_batch() {
        let mut g = game_on(MapId::SplitBasin, Faction::Union);
        let far = put(&mut g, 0, Kind::Riveter, 1, 4, 4);
        let near_pos = ground(&g, 0, 3, 3);
        unit(&mut g, 0, Kind::Riveter, near_pos);
        g.camera.center(near_pos);
        g.selected = vec![far];
        g.key("1", false, true);
        g.render();
        let camera = (g.camera.x, g.camera.y);
        let (x, y) = g.project(near_pos);
        let shown = g.unit_at(x, y - 4).expect("a machine under the pointer");
        assert_ne!(shown, far);
        // One batch read off one picture: the group key twice, then a
        // click on a machine that picture showed.
        g.begin_input_batch();
        g.key("1", false, false);
        g.key("1", false, false);
        assert_eq!((g.camera.x, g.camera.y), camera, "no recentre mid-batch");
        g.pointer_moved(x, y - 4);
        g.left_down(x, y - 4);
        g.left_up(x, y - 4, false);
        assert_eq!(
            g.selected,
            vec![shown],
            "the click took what the picture showed"
        );
        g.end_input_batch();
        assert_ne!(
            (g.camera.x, g.camera.y),
            camera,
            "the recentre came after the batch"
        );
        // Outside a batch the recentre is at once.
        (g.camera.x, g.camera.y) = camera;
        g.key("1", false, false);
        g.key("1", false, false);
        assert_ne!((g.camera.x, g.camera.y), camera);
    }

    #[test]
    fn a_box_at_the_field_edge_takes_machines_whose_feet_are_under_the_dock() {
        let mut g = game_on(MapId::SplitBasin, Faction::Union);
        g.camera.center(hq(&g, 0).1);
        g.render();
        let view = g.world_view();
        let x = view.width / 2;
        // Feet a few pixels under the field's lower edge, the body above.
        let id = g
            .world
            .spawn_for_tests(0, Kind::Riveter, g.unproject(x, view.bottom + 5));
        let (px, py) = g.project(g.world.entities.iter().find(|e| e.id == id).unwrap().pos);
        assert!(py >= view.bottom, "the feet are under the dock");
        let drag = |g: &mut Game| {
            g.pointer_moved(px - 30, 400);
            g.left_down(px - 30, 400);
            g.pointer_moved(px + 30, view.bottom - 1);
            g.left_up(px + 30, view.bottom - 1, false);
        };
        drag(&mut g);
        assert_eq!(g.selected, vec![id]);
        // One wholly under the dock is not taken.
        let under = g
            .world
            .spawn_for_tests(0, Kind::Riveter, g.unproject(px, view.bottom + 80));
        drag(&mut g);
        assert!(!g.selected.contains(&under));
    }

    #[test]
    fn shift_i_leaves_a_worker_just_given_a_site() {
        let mut g = game_on(MapId::SplitBasin, Faction::Union);
        g.world.players[0].salvage = 2000;
        let workers: Vec<u32> = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        for e in g
            .world
            .entities
            .iter_mut()
            .filter(|e| workers.contains(&e.id))
        {
            e.order = Order::Idle;
        }
        let builder = workers[0];
        g.selected = vec![builder];
        let site = ground(&g, 0, 6, -6);
        let (worker, queued) = g.builder_and_queue(site).expect("a builder");
        assert_eq!(worker, builder);
        g.recent_builders.push((worker, g.world.tick));
        assert!(g.issue_accepted(Command::Build {
            worker,
            kind: Kind::Palisade,
            pos: site,
            queued,
        }));
        // Before the order shows on the field the builder may still read
        // idle there; it is not idle to Shift+I.
        g.key("I", true, false);
        assert!(!g.selected.contains(&builder), "{:?}", g.selected);
        assert_eq!(g.selected.len(), workers.len() - 1);
        for _ in 0..10 {
            g.tick();
        }
        assert!(matches!(
            g.world
                .entities
                .iter()
                .find(|e| e.id == builder)
                .unwrap()
                .order,
            Order::Build { .. }
        ));
        g.key("I", true, false);
        assert!(!g.selected.contains(&builder), "on its site, never idle");
    }

    /// The trial's own recording at 33:43 (t60704), where the Union read
    /// "the lane is deep" about a lane its card called DRY: which lanes
    /// were deep, and whether its workers could reach the wrecks of the
    /// dry lane.  `BW_TRIAL12_REPLAY=match.replay.json cargo test
    /// --release -p bw_desktop -- --ignored trial12_replay_at_33_43`.
    #[test]
    #[ignore]
    fn trial12_replay_at_33_43() {
        let Ok(path) = std::env::var("BW_TRIAL12_REPLAY") else {
            return;
        };
        let mut player = bw_sim::ReplayPlayer::open(&path).expect("replay");
        while player.tick() < 60_704 {
            assert!(player.step().expect("step"), "the recording ends early");
        }
        let world = player.world();
        let union = world
            .players
            .iter()
            .position(|p| p.faction == Faction::Union)
            .expect("the Union's seat") as u8;
        println!(
            "tide {:?} dry arm {:?}",
            world.gate.tide, world.gate.dry_arm
        );
        for arm in 0..crate::seats::arm_count(world) {
            println!(
                "arm {} deep {} banks {:?}",
                crate::seats::arm_letter(world, arm),
                crate::lane_wall::lane_closed(world, arm),
                world.arm_banks(arm)
            );
        }
        let workers: Vec<u32> = world
            .entities
            .iter()
            .filter(|e| e.owner == union && e.hp > 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        for arm in 0..crate::seats::arm_count(world) {
            for wreck in world.map.layout().arms[arm].wrecks {
                let pos = Pos::cell(wreck.0, wreck.1);
                let left = world
                    .map
                    .resources
                    .iter()
                    .find(|r| r.pos.cell_xy() == wreck)
                    .map_or(0, |r| r.remaining);
                println!(
                    "{} lane wreck {wreck:?} left {left}: Union workers reach it {}",
                    crate::seats::arm_letter(world, arm),
                    world.any_can_reach_within(&workers, pos, |_| super::BESIDE_WRECK)
                );
            }
        }
    }
}
