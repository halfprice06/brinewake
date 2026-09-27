//! Presentation-only selection, chart commands and interruption recovery.
use crate::game::{Game, Mode, Screen};
use bw_core::{EntityId, FP, Kind, Pos};
use bw_sim::{Command, Order};
use std::path::{Path, PathBuf};

impl Game {
    pub(crate) fn idle_worker_ids(&self) -> Vec<EntityId> {
        let mut ids: Vec<_> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.aboard.is_none()
                    && e.kind.is_worker()
                    && matches!(e.order, Order::Idle)
                    && !self.worker_order_pending(e)
            })
            .map(|e| e.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Own and seen enemy combat machines both stand at the sluice.
    pub(crate) fn sluice_contested(&self) -> bool {
        let gate = self.world.map.gate_pos;
        let radius = i64::from(FP * 2).pow(2);
        let at_gate = |e: &bw_sim::Entity| {
            e.hp > 0
                && e.build_remaining == 0
                && !e.kind.is_worker()
                && !e.kind.is_building()
                && e.pos.distance_sq(gate) <= radius
        };
        let own = self
            .world
            .entities
            .iter()
            .any(|e| e.owner == 0 && at_gate(e));
        let enemy = self
            .world
            .entities
            .iter()
            .any(|e| e.owner != 0 && at_gate(e) && self.world.entity_visible(0, e.id));
        own && enemy
    }

    /// The hold count that is running, if any: whose gauge and its ticks.
    /// A draining gauge is reported as the count it is; the caller says
    /// whether the lanes are held now.
    pub(crate) fn hold_gauge(&self) -> Option<(u8, u32)> {
        let own_holding = self.world.holds_every_lane(0);
        let own = self.world.lane_hold[0];
        // The opponent that counts: one holding now, else the highest gauge.
        // Only the station's owner holds a lane, so one seat at most counts.
        let holder = self
            .world
            .seats()
            .skip(1)
            .find(|&seat| self.world.holds_every_lane(seat));
        let (rival, enemy) = match holder {
            Some(seat) => (seat, self.world.lane_hold[seat as usize]),
            None => self.world.leading_enemy_gauge().unwrap_or((1, 0)),
        };
        let enemy_holding = holder.is_some();
        if own_holding || (own > 0 && !enemy_holding && own >= enemy) {
            Some((0, own))
        } else if enemy_holding || enemy > 0 {
            Some((rival, enemy))
        } else {
            None
        }
    }

    /// Every own combat machine except the mouth keepers: F2 and ARMY
    /// leave a machine standing at a crossing mouth where it is, so
    /// select-all never pulls a tide hold apart.
    pub(crate) fn army_ids(&self) -> Vec<EntityId> {
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0
                    && e.hp > 0
                    && e.aboard.is_none()
                    && crate::trial11_controls::is_army_kind(e.kind)
                    && !keeps_mouth(&self.world, e)
            })
            .map(|e| e.id)
            .collect()
    }

    /// Own machines on HOLD at a crossing mouth: the ones F2 leaves.
    pub(crate) fn mouth_keeper_ids(&self) -> Vec<EntityId> {
        self.world
            .entities
            .iter()
            .filter(|e| {
                e.owner == 0 && e.hp > 0 && e.aboard.is_none() && keeps_mouth(&self.world, e)
            })
            .map(|e| e.id)
            .collect()
    }

    /// The mouth that decides the running hold, for the banner and F3.
    /// Against an enemy count it is the enemy-held mouth nearest home, where
    /// one gun stops the count.  For our own count it is the lane that is
    /// not held yet, else our own mouth nearest home.  With no count
    /// running it is the first lane we do not hold.
    pub(crate) fn hold_focus_mouth(&self) -> Option<Pos> {
        let player = self.hold_gauge().map_or(0, |(player, _)| player);
        let home = self
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .map_or(self.world.map.gate_pos, |e| e.pos);
        let holders = mouth_holders(&self.world);
        let seats = mouth_seats(&self.world);
        let mouths = self.world.crossing_mouths();
        // The lanes the count is about: the counting seat's own (both on
        // the Split Basin, the two beside its land on the Confluence).
        let lanes = self.world.hold_arms(player);
        let nearest = |pick: &dyn Fn(usize, usize) -> bool| {
            lanes
                .iter()
                .flat_map(|&lane| (0..2).map(move |mouth| (lane, mouth)))
                .filter(|(lane, mouth)| pick(*lane, *mouth))
                .map(|(lane, mouth)| mouths[lane][mouth])
                .min_by_key(|pos| (pos.distance_sq(home), pos.x, pos.y))
        };
        if player != 0 {
            return nearest(&|lane, mouth| seats[lane][mouth] & (1 << player) != 0)
                .or(Some(self.world.map.gate_pos));
        }
        if let Some(lane) = lanes
            .iter()
            .copied()
            .find(|lane| !self.world.holds_lane(0, *lane))
        {
            // What blocks it, itself: the nest or machine nearest home
            // (trial 12: the Assembly never found the nests on its E lane).
            if let Some(blocker) = crate::trial12_hold::lane_blockers(&self.world, lane)
                .into_iter()
                .min_by_key(|b| (b.pos.distance_sq(home), b.id))
            {
                return Some(blocker.pos);
            }
            return nearest(&|l, mouth| l == lane && holders[l][mouth].1)
                .or_else(|| nearest(&|l, _| l == lane));
        }
        nearest(&|lane, mouth| holders[lane][mouth].0)
    }

    pub(crate) fn select_recovered(&mut self, ids: Vec<EntityId>, center: bool) {
        self.selected = ids;
        self.selected.sort_unstable();
        self.selected.dedup();
        if center
            && let Some(e) = self
                .world
                .entities
                .iter()
                .find(|e| self.selected.contains(&e.id))
        {
            self.recentre(e.pos);
        }
        self.mode = Mode::Context;
        self.drag = None;
        self.ux.minimap_drag = false;
        self.last_click = None;
        self.last_group = None;
        self.update_tutorial();
    }

    /// Centre the view on the current selection without changing it.
    pub(crate) fn center_on_selection(&mut self) {
        if let Some(e) = self
            .world
            .entities
            .iter()
            .find(|e| self.selected.contains(&e.id))
        {
            self.camera.center(e.pos);
            self.camera_sub = (0.0, 0.0);
            self.notify("Centred on the selection.");
        } else {
            self.notify("Nothing is selected.");
        }
    }

    pub(crate) fn select_idle_workers(&mut self, all: bool) {
        let ids = self.idle_worker_ids();
        if ids.is_empty() {
            // Workers gather on their own and never read as idle, so a
            // builder was found by walking to the wrecks: the nearest
            // worker to the view answers instead.
            let centre = self.unproject(self.world_view().center().0, self.world_view().center().1);
            let nearest = self
                .world
                .entities
                .iter()
                .filter(|e| e.owner == 0 && e.hp > 0 && e.aboard.is_none() && e.kind.is_worker())
                .min_by_key(|e| e.pos.distance_sq(centre))
                .map(|e| e.id);
            match nearest {
                Some(id) => {
                    self.selected = vec![id];
                    self.notify("No idle workers. The nearest worker is selected instead.");
                }
                None => self.notify("No workers."),
            }
            return;
        }
        if all {
            self.select_recovered(ids, false);
        } else {
            // A selection key never moves the view: a select-then-click
            // sequence depends on the picture staying put, and repeated
            // presses cycle. Ctrl+I centres on the selection instead.
            let id = self
                .ux
                .last_idle_worker
                .and_then(|last| ids.iter().copied().find(|id| *id > last))
                .unwrap_or(ids[0]);
            self.ux.last_idle_worker = Some(id);
            self.select_recovered(vec![id], false);
        }
    }

    pub(crate) fn select_army(&mut self, center: bool) {
        let ids = self.army_ids();
        let keepers = self.mouth_keeper_ids().len();
        if ids.is_empty() {
            self.notify(if keepers > 0 {
                "Every combat machine is holding a crossing mouth."
            } else {
                "No combat machines yet. Train them at Works."
            });
            return;
        }
        let count = ids.len();
        self.select_recovered(ids, center);
        // What stays behind is named: the Pan, Salter, scouts and menders
        // walked off with attack-moves in trial 11.
        if keepers > 0 || !self.army_left_out().is_empty() {
            let words = self.army_words(count, keepers);
            self.notify(&words);
        }
    }

    pub(crate) fn control_group(&mut self, n: usize, shift: bool, control: bool) {
        if control {
            self.last_group = None;
            let selected = self.gameplay_ids(|_| true);
            if !shift {
                self.groups[n].clear();
            }
            self.groups[n].extend(selected);
            self.groups[n].sort_unstable();
            self.groups[n].dedup();
        }
        // Groups are UI state and can outlive a member's death.
        self.groups[n].retain(|id| {
            self.world
                .entities
                .iter()
                .any(|e| e.id == *id && e.owner == 0 && e.hp > 0 && e.aboard.is_none())
        });
        if control {
            let count = self.groups[n].len();
            if count == 0 {
                self.notify(&format!("Group {n} cleared."));
            } else {
                self.notify(&format!(
                    "Group {n}: {count} machine{}{}.",
                    if count == 1 { "" } else { "s" },
                    if shift { " after adding" } else { "" }
                ));
            }
        } else if self.groups[n].is_empty() {
            self.notify(&format!(
                "Group {n} is empty. Ctrl+{n} stores the selection."
            ));
        } else {
            let center = shift
                || self.last_group.is_some_and(|(last, frame)| {
                    last == n && self.frame.saturating_sub(frame) < 24
                });
            self.recall_ends_inspection();
            self.select_recovered(self.groups[n].clone(), center);
            self.last_group = Some((n, self.frame));
        }
    }

    pub(crate) fn minimap_pos(&self, x: i32, y: i32) -> Pos {
        // A press on the chart's margin goes to the nearest map edge. This
        // is also the drag clamp.
        let (u, v) = self.chart().pos_at(&self.world, x, y).cell_xy();
        Pos::cell(u, v)
    }

    /// The minimap's chart: the map in its field shape inside the bounds.
    pub(crate) fn chart(&self) -> crate::minimap_chart::ChartRect {
        let r = self.minimap_bounds();
        crate::minimap_chart::ChartRect::field(r.x, r.y, r.w, r.h)
    }

    pub(crate) fn pointer_moved(&mut self, x: i32, y: i32) {
        if (x, y) != self.cursor {
            self.pointer_moved_tick = self.world.tick.saturating_add(self.aftermath_ticks);
            // On a menu the pointer carries focus: one control is lit, and
            // it is the one Enter would press.
            let live = self.screen == Screen::Match
                && self.world.outcome.is_none()
                && !self.ux.practice_review;
            if !live && let Some(b) = self.buttons.iter().find(|b| b.enabled && b.contains(x, y)) {
                self.ux.focused = Some(b.action.clone());
            }
        }
        self.cursor = (x, y);
        if let Some(bus) = self.ux.volume_drag
            && let Some(b) = self
                .buttons
                .iter()
                .find(|b| b.action == crate::game::Action::Volume(bus))
                .cloned()
        {
            self.action(crate::game::Action::VolumeSet(
                bus,
                crate::game::volume_at(&b, x),
            ));
        }
        self.follow_tide_card_hover();
        if self.ux.minimap_drag && self.screen == Screen::Match {
            self.minimap_click(x, y);
        }
    }

    pub(crate) fn pointer_left(&mut self) {
        self.end_volume_drag();
        self.cursor = (-999, -999);
        self.drag = None;
        self.ux.minimap_drag = false;
    }

    pub(crate) fn attack_destination(&mut self, target: Pos) {
        let queued = self.ux.pointer_shift;
        let units = self.gameplay_ids(|k| !k.is_building());
        if units.is_empty() {
            self.notify("Select machines first.");
            return;
        }
        // An attack-move nobody could carry out is refused in `issue`.
        self.issue(Command::AttackMove {
            units,
            target,
            queued,
        });
        if queued {
            self.mode = Mode::Attack;
        }
    }

    pub(crate) fn minimap_order(&mut self, x: i32, y: i32, queued: bool) {
        let target = self.minimap_pos(x, y);
        let units = self.gameplay_ids(|k| !k.is_building());
        if !units.is_empty() {
            self.issue(Command::Move {
                units,
                target,
                queued,
            });
        } else if self.rally_selected(target) {
            // Every selected building that trains, the Drydock included.
        } else {
            self.notify("Select machines to move, or a building to set its rally.");
        }
    }

    pub(crate) fn focus_lost(&mut self) {
        self.pointer_left();
        self.ux.pointer_shift = false;
        self.last_click = None;
        self.last_group = None;
        if self.ux.preferences.pause_unfocused
            && self.screen == Screen::Match
            && self.world.outcome.is_none()
            && !self.ux.practice_review
        {
            self.open_screen(Screen::Pause);
            self.ux.paused_unfocused = true;
            self.notify("Paused while you were away.");
        }
    }
}

/// Ordinary starting fields and real commands, with controlled UI input.
pub(crate) fn export(base: PathBuf, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for faction in [bw_core::Faction::Union, bw_core::Faction::Assembly] {
        let name = if faction == bw_core::Faction::Union {
            "union"
        } else {
            "assembly"
        };
        let mut g = Game::new_with_data_dir(base.clone(), dir.join(format!("{name}-profile")));
        g.faction = faction;
        g.begin_practice();
        g.intro = None;
        g.ux.guidance_visible = false;
        g.resize_view(1920, 1080);
        g.zoom = crate::zoom::Zoom::Wide;
        g.selected = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        g.action(crate::game::Action::Stop);
        g.tick();
        let idle = g.idle_worker_ids();
        if idle.len() != 6 {
            return Err(format!("{name}: expected six idle starting workers"));
        }
        g.key("I", false, false);
        let first = g.selected.clone();
        g.key("I", false, false);
        if g.selected == first {
            return Err("idle cycling did not advance".into());
        }
        g.screenshot(&dir.join(format!("{name}-idle.png")))?;
        g.key("I", true, false);
        if g.selected != idle {
            return Err("all-idle selection mismatch".into());
        }
        g.key("1", false, true);
        g.key("1", false, false);
        let r = g.minimap_bounds();
        let chart = |u: i32, v: i32| {
            (
                r.x + (u * (r.w - 1) + 127) / 128,
                r.y + (v * (r.h - 1) + 127) / 128,
            )
        };
        let a = chart(23, 64);
        let b = chart(28, 64);
        g.right_click(a.0, a.1, false);
        g.right_click(b.0, b.1, true);
        g.action(crate::game::Action::Attack);
        g.ux.pointer_shift = true;
        let c = chart(31, 63);
        g.left_down(c.0, c.1);
        g.left_up(c.0, c.1, true);
        if g.mode != Mode::Attack {
            return Err("queued attack mode was not retained".into());
        }
        g.ux.pointer_shift = false;
        g.key("Escape", false, false);
        g.tick();
        if g.world
            .command_log
            .iter()
            .any(|r| !r.accepted || r.applied == Some(false))
        {
            return Err(format!("{name}: QoL command rejected"));
        }
        if !g
            .world
            .entities
            .iter()
            .filter(|e| idle.contains(&e.id))
            .all(|e| e.waypoints.len() == 2)
        {
            return Err("real queued waypoints were not preserved".into());
        }
        g.screenshot(&dir.join(format!("{name}-queued-chart.png")))?;
        g.home();
        g.right_click(a.0, a.1, false);
        g.tick();
        g.left_down(a.0, a.1);
        g.pointer_moved(b.0, b.1);
        g.left_up(b.0, b.1, false);
        if g.ux.minimap_drag {
            return Err("chart drag stuck after release".into());
        }
        g.focus_lost();
        let paused_hash = g.world.state_hash();
        for _ in 0..120 {
            g.tick();
        }
        if g.screen != Screen::Pause || g.world.state_hash() != paused_hash {
            return Err("away pause advanced simulation".into());
        }
        g.screenshot(&dir.join(format!("{name}-away.png")))?;
        g.action(crate::game::Action::Resume);
        for _ in 0..90 {
            g.tick();
        }
        let replay_path = dir.join(format!("{name}.replay.json"));
        g.world.export_replay(&replay_path)?;
        let hash = g.world.state_hash();
        if bw_sim::World::replay(&replay_path)?.state_hash() != hash {
            return Err("QoL replay diverged".into());
        }
        if !g.save_match() {
            return Err("QoL save failed".into());
        }
        g.load_match();
        if g.world.state_hash() != hash {
            return Err("QoL load diverged".into());
        }
        g.home();
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.cursor = (-999, -999);
            g.screenshot(&dir.join(format!("{name}-{w}x{h}.png")))?;
            let button = g
                .buttons
                .iter()
                .find(|b| b.action == crate::game::Action::SelectArmy)
                .unwrap()
                .clone();
            g.cursor = (button.x + 3, button.y + 3);
            g.screenshot(&dir.join(format!("{name}-tooltip-{w}x{h}.png")))?;
        }
        g.action(crate::game::Action::FocusWorker);
        g.action(crate::game::Action::Build(Kind::Works));
        g.cursor = g.project(
            g.world
                .entities
                .iter()
                .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
                .unwrap()
                .pos,
        );
        g.notify("Old notification must not hide placement guidance.");
        g.screenshot(&dir.join(format!("{name}-placement.png")))?;
        g.action(crate::game::Action::Settings);
        g.screenshot(&dir.join(format!("{name}-settings.png")))?;
        results.push(serde_json::json!({"faction":name,"ticks":g.world.tick,"commands":g.world.command_log.len(),"hash":hash,"replay_matches":true,"save_matches":true,"scope":"Controlled ordinary-start UI journey; no human play judgment"}));
    }
    std::fs::write(
        dir.join("journeys.json"),
        serde_json::to_vec_pretty(&results).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// A machine standing still inside a crossing mouth's ring keeps that
/// mouth: on HOLD, idle, deployed, or at the end of its move.  In the ninth
/// trial the Assembly's holders arrived by attack-move and stood idle, and
/// F2 pulled them off twice; only HOLD counted then.
pub(crate) fn keeps_mouth(world: &bw_sim::World, e: &bw_sim::Entity) -> bool {
    let radius = i64::from(bw_core::FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let standing = match e.order {
        Order::Hold | Order::Idle | Order::Deploy => true,
        Order::Move { .. } | Order::AttackMove { .. } => {
            e.path.is_empty() && e.waypoints.is_empty()
        }
        _ => false,
    };
    standing
        && lane_holder(e.kind)
        && world
            .crossing_mouths()
            .iter()
            .flatten()
            .any(|mouth| e.pos.distance_sq(*mouth) <= radius)
}

/// Per arm and mouth (in the arm's mouth order): the seats whose holders
/// stand in its ring, one bit per seat.  The mouths are lit for everyone.
pub(crate) fn mouth_seats(world: &bw_sim::World) -> Vec<[u8; 2]> {
    let radius = i64::from(bw_core::FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let mouths = world.crossing_mouths();
    let mut out = vec![[0u8; 2]; mouths.len()];
    for e in &world.entities {
        if e.hp <= 0 || e.build_remaining > 0 || e.aboard.is_some() || !lane_holder(e.kind) {
            continue;
        }
        for (lane, pair) in mouths.iter().enumerate() {
            for (index, mouth) in pair.iter().enumerate() {
                if e.pos.distance_sq(*mouth) <= radius {
                    out[lane][index] |= 1u8 << e.owner.min(7);
                }
            }
        }
    }
    out
}

/// Per arm and mouth: the opponents (never seat 0) whose finished Defense
/// Nest stands in its ring, one bit per seat.  Since rules 20 such a nest
/// blocks seat 0's count there, though it never holds a mouth for its own
/// side; with three seats (rules 22) it only slows the count.
pub(crate) fn mouth_nests(world: &bw_sim::World) -> Vec<[u8; 2]> {
    let radius = i64::from(bw_core::FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let mouths = world.crossing_mouths();
    let mut out = vec![[0u8; 2]; mouths.len()];
    for e in &world.entities {
        if e.owner == 0
            || e.hp <= 0
            || e.build_remaining > 0
            || !bw_sim::blocks_mouth_building(e.kind)
        {
            continue;
        }
        let middle = bw_sim::footprint_middle(e);
        for (lane, pair) in mouths.iter().enumerate() {
            for (index, mouth) in pair.iter().enumerate() {
                if middle.distance_sq(*mouth) <= radius {
                    out[lane][index] |= 1u8 << e.owner.min(7);
                }
            }
        }
    }
    out
}

/// Per arm and mouth (in the arm's mouth order; west then east on the
/// Split Basin): whether an own holder stands in its ring, and whether an
/// enemy holder or enemy nest blocks it.  The mouths are lit for both
/// sides.
pub(crate) fn mouth_holders(world: &bw_sim::World) -> Vec<[(bool, bool); 2]> {
    mouth_seats(world)
        .into_iter()
        .zip(mouth_nests(world))
        .map(|(pair, nests)| {
            // With three seats a nest slows a count and blocks nothing
            // (rules 22).
            let nests = if world.nests_slow_counts() {
                [0, 0]
            } else {
                nests
            };
            [0, 1].map(|i| (pair[i] & 1 != 0, (pair[i] | nests[i]) & !1 != 0))
        })
        .collect()
}

/// The seat that holds an arm's lane, if one does: only the station's
/// owner can.
pub(crate) fn lane_holder_seat(world: &bw_sim::World, arm: usize) -> Option<u8> {
    world
        .gate
        .owner
        .filter(|&owner| world.holds_lane(owner, arm))
}

/// The colour of a seat's hold on the tide marks: gold for your own, as
/// the sluice card has always drawn it, then each opponent's own colour.
pub(crate) fn hold_color(world: &bw_sim::World, seat: u8) -> crate::canvas::Color {
    if seat == 0 {
        crate::canvas::GOLD
    } else {
        crate::seats::seat_colour(world, seat)
    }
}

/// What counts at a crossing mouth: every gun machine, and the Caisson,
/// whose whole role is to hold one.  Mirrors the simulation's rule.
pub(crate) fn lane_holder(kind: bw_core::Kind) -> bool {
    !kind.is_worker()
        && !kind.is_building()
        && (bw_content::spec(kind).damage > 0 || kind == bw_core::Kind::Caisson)
}

/// Who stands at a lane's mouths: whether an own holder does, and the mouth
/// the nearest enemy holder stands at.  The mouths are lit for both sides,
/// so this reads the world directly.
pub(crate) fn lane_presence(world: &bw_sim::World, lane: usize) -> (bool, Option<Pos>) {
    let mouths = world.map.layout().arm_mouths(lane);
    let radius = i64::from(bw_core::FP * bw_content::CROSSING_HOLD_RADIUS_CELLS).pow(2);
    let mut own = false;
    let mut enemy: Option<(i64, Pos)> = None;
    for e in &world.entities {
        // An enemy nest in the ring blocks a count as a gun does (rules 20),
        // on two seats only: with three it slows the count (rules 22).
        let place = if lane_holder(e.kind) {
            e.pos
        } else if e.owner != 0
            && bw_sim::blocks_mouth_building(e.kind)
            && !world.nests_slow_counts()
        {
            bw_sim::footprint_middle(e)
        } else {
            continue;
        };
        if e.hp <= 0 || e.build_remaining > 0 || e.aboard.is_some() {
            continue;
        }
        for mouth in mouths {
            let d = place.distance_sq(mouth);
            if d > radius {
                continue;
            }
            if e.owner == 0 {
                own = true;
            } else if enemy.is_none_or(|(best, _)| d < best) {
                enemy = Some((d, mouth));
            }
        }
    }
    (own, enemy.map(|(_, mouth)| mouth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Action;
    use bw_core::Faction;

    fn game(label: &str) -> Game {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut g = Game::new_with_data_dir(
            base,
            std::env::temp_dir().join(format!("bw-qol-{label}-{}", std::process::id())),
        );
        g.begin_practice();
        g.intro = None;
        g.ux.guidance_visible = false;
        g.resize_view(1920, 1080);
        g.cursor = (-999, -999);
        g
    }

    #[test]
    fn idle_recovery_cycles_live_friends_and_all_preserves_camera_and_empty_selection() {
        for faction in [Faction::Union, Faction::Assembly] {
            let mut g = game("idle");
            g.faction = faction;
            g.begin_practice();
            g.intro = None;
            let workers: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.owner == 0 && e.kind.is_worker())
                .map(|e| e.id)
                .collect();
            for e in &mut g.world.entities {
                if e.kind.is_worker() {
                    e.order = Order::Idle;
                }
                if e.id == workers[0] {
                    e.hp = 0;
                }
                if e.id == workers[1] {
                    e.order = Order::Gather { resource: 0 };
                }
            }
            let expected = workers[2..].to_vec();
            assert_eq!(g.idle_worker_ids(), expected);
            let hash = g.world.state_hash();
            let resting = (g.camera.x, g.camera.y);
            for &id in expected.iter().chain(expected.iter().take(1)) {
                g.key("I", false, false);
                assert_eq!(g.selected, vec![id]);
                assert_eq!(
                    (g.camera.x, g.camera.y),
                    resting,
                    "selecting leaves the view"
                );
            }
            g.key("I", false, true);
            let centred = g.selected[0];
            assert_eq!(
                g.unproject(g.world_view().center().0, g.world_view().center().1),
                g.world
                    .entities
                    .iter()
                    .find(|e| e.id == centred)
                    .unwrap()
                    .pos,
                "a quick second tap centres on the selected worker"
            );
            let camera = (g.camera.x, g.camera.y);
            g.mode = Mode::Build(Kind::Works);
            g.key("I", true, false);
            assert_eq!(g.selected, expected);
            assert_eq!((g.camera.x, g.camera.y), camera);
            assert_eq!(g.mode, Mode::Context);
            assert_eq!(g.world.state_hash(), hash);
            for e in &mut g.world.entities {
                if e.kind.is_worker() {
                    e.order = Order::Hold;
                }
            }
            g.key("I", false, false);
            assert_eq!(g.selected.len(), 1, "the nearest worker answers instead");
            assert!(g.message.contains("No idle"));
        }
    }

    #[test]
    fn group_reinforcement_and_explicit_army_selection_filter_stale_members() {
        let mut g = game("groups");
        let template = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap()
            .clone();
        for (id, owner, hp) in [(100, 0, 50), (101, 0, 50), (102, 0, 0), (103, 1, 50)] {
            let mut e = template.clone();
            e.id = id;
            e.owner = owner;
            e.hp = hp;
            e.kind = Kind::Riveter;
            g.world.entities.push(e);
        }
        let hash = g.world.state_hash();
        let camera = (g.camera.x, g.camera.y);
        g.key("F2", false, false);
        assert_eq!(g.selected, vec![100, 101]);
        assert_eq!((g.camera.x, g.camera.y), camera);
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.render();
            for action in [Action::Formation, Action::Face] {
                let b = g.buttons.iter().find(|b| b.action == action).unwrap();
                assert!(
                    b.y >= g.world_view().bottom + 46 * g.ui_scale(),
                    "tactical controls live in the command card, not over the field"
                );
            }
        }
        g.selected = vec![100];
        g.key("1", false, true);
        g.selected = vec![101, 101, 102, 103, 999];
        g.key("1", true, true);
        assert_eq!(g.groups[1], vec![100, 101]);
        g.groups[1].extend([102, 103, 999]);
        g.selected.clear();
        g.key("1", false, false);
        assert_eq!(g.selected, vec![100, 101]);
        assert_eq!(g.groups[1], vec![100, 101]);
        g.key("9", false, false);
        assert_eq!(g.selected, vec![100, 101]);
        assert!(g.message.contains("empty"));
        assert_eq!(g.world.state_hash(), hash);
    }

    #[test]
    fn shift_box_includes_workers_and_shift_double_click_preserves_other_types() {
        let mut g = game("selection");
        g.ux.practice = false;
        let worker = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap();
        worker.pos = Pos::cell(30, 61);
        let id = worker.id;
        let mut combat = worker.clone();
        combat.id = 100;
        combat.kind = Kind::Riveter;
        combat.pos = Pos::cell(34, 61);
        g.world.entities.push(combat.clone());
        combat.id = 101;
        combat.pos = Pos::cell(38, 61);
        g.world.entities.push(combat.clone());
        combat.id = 102;
        combat.pos = Pos::cell(110, 10);
        g.world.entities.push(combat.clone());
        combat.id = 103;
        combat.hp = 0;
        combat.pos = Pos::cell(36, 61);
        g.world.entities.push(combat);
        g.camera.center(Pos::cell(34, 61));
        g.render();
        let p = g.project(Pos::cell(30, 61));
        let q = g.project(Pos::cell(38, 61));
        let drag = |g: &mut Game, shift| {
            g.left_down(p.0 - 25, p.1 - 50);
            g.left_up(q.0 + 25, q.1 + 5, shift);
        };
        drag(&mut g, false);
        assert_eq!(g.selected, vec![100, 101]);
        g.selected.clear();
        drag(&mut g, true);
        assert_eq!(g.selected, vec![id, 100, 101]);
        let (x, y) = g.project(Pos::cell(34, 61));
        let hit = (-60..0)
            .flat_map(|dy| (-30..30).map(move |dx| (x + dx, y + dy)))
            .find(|&(x, y)| g.unit_at(x, y) == Some(100))
            .unwrap();
        g.selected = vec![id];
        g.last_click = Some((100, g.frame));
        g.left_down(hit.0, hit.1);
        g.left_up(hit.0, hit.1, true);
        assert_eq!(g.selected, vec![id, 100, 101]);
    }

    #[test]
    fn chart_drag_clamps_releases_and_does_not_issue_world_orders() {
        let mut g = game("chart-drag");
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.render();
            let r = g.minimap_bounds();
            let c = g.chart();
            let commands = g.world.command_log.len();
            // The chart is the field's diamond: north-west at its top point,
            // south-east at its bottom point, the middle in the middle.
            assert_eq!(g.minimap_pos(c.x + c.w / 2, c.y).cell_xy(), (0, 0));
            assert_eq!(
                g.minimap_pos(c.x + c.w / 2, c.y + c.h).cell_xy(),
                (127, 127)
            );
            assert_eq!(
                g.minimap_pos(r.x + r.w / 2, r.y + r.h / 2),
                Pos::cell(64, 64)
            );
            g.left_down(r.x + r.w / 2, r.y + 20);
            assert!(g.ux.minimap_drag);
            g.pointer_moved(r.x + r.w / 2, r.y + r.h + 100);
            assert_eq!(
                g.unproject(g.world_view().center().0, g.world_view().center().1)
                    .cell_xy(),
                (127, 127)
            );
            g.left_up(r.x + r.w / 2, r.y + r.h + 100, false);
            let camera = (g.camera.x, g.camera.y);
            g.pointer_moved(800, 400);
            assert_eq!((g.camera.x, g.camera.y), camera);
            assert_eq!(g.world.command_log.len(), commands);
            g.left_down(r.x + r.w / 2, r.y + 20);
            g.pointer_left();
            assert!(!g.ux.minimap_drag);
        }
    }

    #[test]
    fn chart_commands_queue_ground_orders_without_resolving_fog_and_cancel_modes() {
        let mut g = game("chart-orders");
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap()
            .id;
        let enemy = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 1 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos
            .cell_xy();
        g.selected = vec![worker];
        g.render();
        let r = g.minimap_bounds();
        let p = (r.x + enemy.0 * r.w / 128, r.y + enemy.1 * r.h / 128);
        g.right_click(p.0, p.1, true);
        assert!(matches!(
            g.world.command_log.last().unwrap().command,
            Command::Move { queued: true, .. }
        ));
        g.action(Action::Attack);
        g.ux.pointer_shift = true;
        g.left_down(p.0, p.1);
        g.left_up(p.0, p.1, true);
        assert!(matches!(
            g.world.command_log.last().unwrap().command,
            Command::AttackMove { queued: true, .. }
        ));
        assert_eq!(g.mode, Mode::Attack);
        assert!(!g.ux.minimap_drag);
        let count = g.world.command_log.len();
        g.right_click(p.0, p.1, false);
        assert_eq!(g.mode, Mode::Context);
        assert_eq!(g.world.command_log.len(), count);
        g.home();
        g.right_click(p.0, p.1, false);
        assert!(matches!(
            g.world.command_log.last().unwrap().command,
            Command::Rally { .. }
        ));
        let count = g.world.command_log.len();
        for mode in [Mode::Build(Kind::Works), Mode::Gather, Mode::Face] {
            g.mode = mode;
            g.left_down(p.0, p.1);
            assert_eq!(g.world.command_log.len(), count);
            assert_eq!(g.mode, mode);
        }
        g.screen = Screen::Pause;
        g.right_click(p.0, p.1, false);
        assert_eq!(g.world.command_log.len(), count);
    }

    #[test]
    fn shift_attack_on_battlefield_retains_waypoint_mode_and_plain_click_exits() {
        let mut g = game("field-queue");
        let worker = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind.is_worker())
            .unwrap()
            .id;
        g.selected = vec![worker];
        g.render();
        let p = g.world_view().center();
        g.action(Action::Attack);
        g.ux.pointer_shift = true;
        g.left_down(p.0, p.1);
        assert!(matches!(
            g.world.command_log.last().unwrap().command,
            Command::AttackMove { queued: true, .. }
        ));
        assert_eq!(g.mode, Mode::Attack);
        g.ux.pointer_shift = false;
        g.left_down(p.0 + 20, p.1);
        assert!(matches!(
            g.world.command_log.last().unwrap().command,
            Command::AttackMove { queued: false, .. }
        ));
        assert_eq!(g.mode, Mode::Context);
    }

    #[test]
    fn focus_pause_is_opt_out_persistent_and_does_not_replace_other_screens() {
        let mut g = game("focus");
        let legacy: crate::ux::Preferences = serde_json::from_str(r#"{"muted":true}"#).unwrap();
        assert!(legacy.pause_unfocused);
        let hash = g.world.state_hash();
        g.drag = Some((10, 10));
        g.ux.minimap_drag = true;
        g.ux.pointer_shift = true;
        g.focus_lost();
        assert_eq!(g.screen, Screen::Pause);
        assert!(g.ux.paused_unfocused);
        assert!(g.drag.is_none() && !g.ux.minimap_drag && !g.ux.pointer_shift);
        for _ in 0..90 {
            g.tick();
        }
        assert_eq!(g.world.state_hash(), hash);
        g.key("Escape", false, false);
        assert_eq!(g.screen, Screen::Match);
        assert!(!g.ux.paused_unfocused);
        g.action(Action::ToggleFocusPause);
        assert!(
            !crate::ux::UxState::load(&g.data_dir)
                .preferences
                .pause_unfocused
        );
        g.focus_lost();
        assert_eq!(g.screen, Screen::Match);
        g.action(Action::ToggleFocusPause);
        for screen in [
            Screen::Help,
            Screen::Settings,
            Screen::Confirm,
            Screen::Menu,
        ] {
            g.screen = screen;
            g.focus_lost();
            assert_eq!(g.screen, screen);
        }
        g.screen = Screen::Match;
        g.ux.practice_review = true;
        g.focus_lost();
        assert_eq!(g.screen, Screen::Match);
        let _ = std::fs::remove_dir_all(&g.data_dir);
    }

    #[test]
    fn native_recovery_buttons_tooltips_and_placement_are_readable_at_all_sizes() {
        let mut g = game("feedback");
        g.selected = g
            .world
            .entities
            .iter()
            .filter(|e| e.owner == 0 && e.kind.is_worker())
            .map(|e| e.id)
            .collect();
        g.action(Action::Stop);
        g.tick();
        for (w, h) in [(960, 540), (1280, 720), (1537, 865), (1920, 1080)] {
            g.resize_view(w, h);
            g.render();
            let idle = g
                .buttons
                .iter()
                .find(|b| b.action == Action::IdleWorker)
                .unwrap()
                .clone();
            assert!(idle.enabled);
            assert!(idle.label.contains("6"));
            assert!(!g.world_pointer_allowed(idle.x + 1, idle.y + 1));
            g.cursor = (idle.x + 1, idle.y + 1);
            g.render();
            let r = g.native_tooltip_bounds().unwrap();
            // A dock button's tooltip stands on the dock above it, inside
            // the window.
            assert!(
                r.x >= 0 && r.x + r.w <= w as i32 && r.y >= 0 && r.y + r.h <= g.world_view().bottom,
                "{w} {h}: tooltip at {} {} {} {}",
                r.x,
                r.y,
                r.w,
                r.h
            );
            assert!(r.h < 160 * g.ui_scale());
            g.ux.pointer_shift = true;
            g.left_down(idle.x + 1, idle.y + 1);
            g.left_up(idle.x + 1, idle.y + 1, true);
            assert_eq!(g.selected.len(), 6);
            g.ux.pointer_shift = false;
        }
        g.action(Action::Build(Kind::Works));
        let hq = g
            .world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .unwrap()
            .pos;
        g.cursor = g.project(hq);
        g.notify("STALE FEEDBACK");
        let reason = g.placement_reason(Kind::Works, hq).unwrap();
        assert!(g.native_prompt().contains(&reason));
        assert!(!g.native_prompt().contains("STALE"));
    }
}
