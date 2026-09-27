//! Deterministic first-slice navigation and spatial interaction helpers.
//!
//! This module intentionally keeps the first playable small: a bounded grid
//! A* search, footprint-aware static blockers, and deterministic local yielding
//! are enough for the 128x128 basin. The search is complete within a bounded
//! node budget per request; a future resumable frontier can be added to the
//! serialized entity state without changing the public order contract.

use super::*;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

const NAV_EXPANSION_LIMIT: usize = 16_384;
const NAV_REPLAN_AFTER_BLOCKED: u32 = 30;
/// Ten seconds of traffic with no step taken: the machine stops and says
/// so, instead of standing on an order it cannot carry out in silence.
const NAV_GIVE_UP_AFTER_BLOCKED: u32 = 300;
const NAV_DETOUR_AFTER_BLOCKED: u32 = 4;
/// A machine held up by others standing in its way looks for a way around
/// them within this many cells before it tries a one-cell side step.
const NAV_WAY_AROUND_RADIUS: i32 = 6;
const NAV_WAY_AROUND_EXPANSIONS: usize = 4_096;
const NAV_MAX_DESTINATION_RADIUS: i32 = 8;
/// A destination in deep water or rock resolves to the nearest ground
/// within this many cells, so a click on the chart's water still moves.
const NAV_GOAL_RESOLUTION_RADIUS: i32 = 24;
/// A finished machine leaves by the nearest free cell on any side; a crowd
/// at the door is walked around before production is called blocked.
const SPAWN_SEARCH_RADIUS: i32 = 12;
/// Eight-way search (rules 16): the four sides first, then the corners. A
/// corner step costs 141 per 100 of a side step and is taken only when both
/// cells it passes between are open, so nothing cuts a wall or cliff corner.
const NAV_NEIGHBORS: [(i32, i32); 8] = [
    (0, -1),
    (-1, 0),
    (1, 0),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];
const NAV_DIAGONAL_COST: i32 = 141;
const NAV_DETOUR_NEIGHBORS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NavNode {
    f: i32,
    g: i32,
    x: i32,
    y: i32,
}

impl Ord for NavNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap. Reverse every key so the lowest f/g and
        // then row-major cell wins every tie.
        (other.f, other.g, other.y, other.x).cmp(&(self.f, self.g, self.y, self.x))
    }
}

impl PartialOrd for NavNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopRule {
    Exact {
        target: Pos,
    },
    Weapon {
        target: Pos,
        range: i32,
        min_range: i32,
    },
    Gate,
    Interaction,
    /// Beside an own transport: boarding happens in `update_boarding`.
    Board {
        transport: u32,
    },
}

impl World {
    pub(crate) fn ensure_path(&mut self, id: u32, goal: Pos) {
        let Some(snapshot) = self.entity(id).cloned() else {
            return;
        };
        let lane_revision = self.gate.lane_revision;
        let start = snapshot.pos.cell_xy();
        let effective_goal = self.resolve_goal(goal, snapshot.kind);
        let goal_cell = effective_goal.cell_xy();
        let at_goal = snapshot.pos == effective_goal;

        // A completed path is valid at its goal. An in-progress path stays
        // cached until a gate revision or a long traffic wait invalidates it.
        if snapshot.path_target == Some(effective_goal)
            && snapshot.path_lane_revision == lane_revision
            && (at_goal || snapshot.blocked_ticks < NAV_REPLAN_AFTER_BLOCKED)
        {
            return;
        }

        if at_goal {
            if let Some(entity) = self.entity_mut(id) {
                entity.path.clear();
                entity.path_index = 0;
                entity.path_target = Some(effective_goal);
                entity.path_lane_revision = lane_revision;
                entity.blocked_ticks = 0;
            }
            return;
        }

        let path_cells = self.find_path(start, goal_cell, snapshot.kind);
        if path_cells.is_empty() && start != goal_cell {
            // No way through: the machine stops and says so once, instead
            // of standing with an order it can never carry out.
            let owner = snapshot.owner;
            self.clear_order(id);
            self.events.push(Event {
                tick: self.tick,
                kind: EventKind::PathBlocked,
                player: Some(owner),
                entity: Some(id),
                other: None,
                from: Some(snapshot.pos),
                to: Some(goal),
                amount: 0,
                text: "no path".to_string(),
                cause: None,
            });
            return;
        }
        let mut path: Vec<Pos> = path_cells
            .into_iter()
            .map(|(x, y)| Pos::cell(x, y))
            .collect();
        if (start == goal_cell && snapshot.pos != effective_goal)
            || (!path.is_empty()
                && path
                    .last()
                    .is_some_and(|position| *position != effective_goal))
        {
            path.push(effective_goal);
        }
        if let Some(entity) = self.entity_mut(id) {
            entity.path = path;
            entity.path_index = 0;
            entity.path_target = Some(effective_goal);
            entity.path_lane_revision = lane_revision;
            entity.blocked_ticks = 0;
        }
    }

    pub(crate) fn update_navigation(&mut self) {
        let current_lane_revision = self.gate.lane_revision;
        let mut ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && entity.build_remaining == 0
                    && !entity.kind.is_building()
                    && entity.deploy_remaining == 0
                    && !entity.deployed
                    && entity.aboard.is_none()
            })
            .map(|entity| entity.id)
            .collect();
        ids.sort_unstable();

        // (id, next cell, requested destination, blocked age, stop policy).
        let mut desired = Vec::new();
        // Nothing this loop does moves a building or the water, so the
        // areas machines can reach hold for the whole of it.
        let mut reach = Reach::default();
        for id in ids {
            let Some(snapshot) = self.entity(id).cloned() else {
                continue;
            };
            let Some((requested, stop_rule)) = self.navigation_target(&snapshot, &mut reach) else {
                self.clear_navigation_path(id);
                continue;
            };

            if self.stop_reached(&snapshot, stop_rule) {
                self.clear_navigation_path(id);
                continue;
            }

            let target = self.group_destination(id, requested, &snapshot.order, snapshot.kind);
            self.ensure_path(id, target);
            let Some(after) = self.entity(id).cloned() else {
                continue;
            };
            if let Some(next) = after.path.get(after.path_index).copied() {
                desired.push((id, next.cell_xy(), target, after.blocked_ticks, after.kind));
            } else if after.path_target == Some(target) && after.pos.cell_xy() != target.cell_xy() {
                // A failed bounded search or a traffic wait must not cause a
                // full A* recomputation every tick.
                if let Some(entity) = self.entity_mut(id) {
                    entity.blocked_ticks = entity.blocked_ticks.saturating_add(1);
                }
            }
        }

        // Older blocked requests age ahead of fresh requests; entity ID is the
        // stable tie-break. This is deterministic and prevents starvation.
        desired.sort_by_key(|(id, _, _, blocked, _)| (Reverse(*blocked), *id));
        let mut reserved: Vec<(u32, (i32, i32))> = Vec::with_capacity(desired.len());
        for (id, next, target, blocked, kind) in desired {
            // Flight shares the air: nothing on the ground reserves against it.
            let flying = matches!(spec(kind).movement, Movement::Air);
            let already_reserved = !flying
                && reserved.iter().any(|(other_id, cell)| {
                    *cell == next && !self.soft_gather_collision(id, *other_id)
                });
            // Two machines standing corner to corner close the gap between
            // them: nobody squeezes diagonally through it.
            let pinched = !flying
                && self.entity(id).is_some_and(|entity| {
                    let (x, y) = entity.pos.cell_xy();
                    (next.0 - x).abs() == 1
                        && (next.1 - y).abs() == 1
                        && self.occupied_by_other(id, (next.0, y))
                        && self.occupied_by_other(id, (x, next.1))
                });
            if already_reserved || pinched || self.occupied_by_other(id, next) {
                if blocked >= NAV_DETOUR_AFTER_BLOCKED
                    && blocked % NAV_DETOUR_AFTER_BLOCKED == 0
                    && (self.assign_way_around(id, next, target, kind, blocked)
                        || self.assign_detour(id, next, target, kind, blocked))
                {
                    continue;
                }
                let mut give_up = false;
                if let Some(entity) = self.entity_mut(id) {
                    entity.blocked_ticks = entity.blocked_ticks.saturating_add(1);
                    if entity.blocked_ticks >= NAV_GIVE_UP_AFTER_BLOCKED {
                        give_up = true;
                    } else if entity.blocked_ticks >= NAV_REPLAN_AFTER_BLOCKED {
                        // Keep the order and destination but force a bounded
                        // replan on the next pass.
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = Some(target);
                        entity.path_lane_revision = current_lane_revision;
                    }
                }
                if give_up {
                    let owner = self.entity(id).map(|entity| entity.owner);
                    let from = self.entity(id).map(|entity| entity.pos);
                    self.clear_order(id);
                    self.events.push(Event {
                        tick: self.tick,
                        kind: EventKind::PathBlocked,
                        player: owner,
                        entity: Some(id),
                        other: None,
                        from,
                        to: Some(target),
                        amount: 0,
                        text: "blocked".to_string(),
                        cause: None,
                    });
                }
                continue;
            }

            let Some(snapshot) = self.entity(id).cloned() else {
                continue;
            };
            let speed = self.speed_per_tick(&snapshot);
            if speed <= 0 {
                continue;
            }
            let Some(next_pos) = snapshot.path.get(snapshot.path_index).copied() else {
                continue;
            };
            let (new_pos, arrived) = move_towards(snapshot.pos, next_pos, speed);
            let detour =
                snapshot.path.len() == 1 && snapshot.blocked_ticks >= NAV_DETOUR_AFTER_BLOCKED;
            if let Some(entity) = self.entity_mut(id) {
                entity.pos = new_pos;
                if new_pos.x != snapshot.pos.x || new_pos.y != snapshot.pos.y {
                    entity.facing = direction_octant(
                        next_pos.x.saturating_sub(snapshot.pos.x),
                        next_pos.y.saturating_sub(snapshot.pos.y),
                    );
                }
                if !detour {
                    entity.blocked_ticks = 0;
                }
                if arrived {
                    entity.pos = next_pos;
                    entity.path_index = entity.path_index.saturating_add(1);
                    if entity.path_index >= entity.path.len() {
                        entity.path.clear();
                        entity.path_index = 0;
                        entity.path_target = None;
                        if let Some(waypoint) = entity.waypoints.first().copied() {
                            entity.waypoints.remove(0);
                            entity.order = match entity.order {
                                Order::AttackMove { .. } => Order::AttackMove { target: waypoint },
                                _ => Order::Move { target: waypoint },
                            };
                        }
                    }
                }
            }
            reserved.push((id, next));
        }
    }

    /// Machines standing on top of each other step apart (rules 14).  In
    /// the eighth trial melee at the station piled three machines onto one
    /// cell and nobody could tell who was fighting whom.  A standing ground
    /// machine closer than three quarters of a cell to another steps to the
    /// nearest free neighbouring cell; of two standing machines only the
    /// higher id moves.  Moving, deployed, holding and working machines stay.
    pub(crate) fn update_spread(&mut self) {
        let near = i64::from(FP * 3 / 4).pow(2);
        let grounded = |entity: &Entity| {
            entity.hp > 0
                && entity.aboard.is_none()
                && !entity.kind.is_building()
                && !matches!(spec(entity.kind).movement, Movement::Air)
        };
        let standing = |entity: &Entity| {
            grounded(entity)
                && entity.path.is_empty()
                && !entity.deployed
                && entity.deploy_remaining == 0
                && matches!(
                    entity.order,
                    Order::Idle
                        | Order::Attack { .. }
                        | Order::AttackMove { .. }
                        | Order::Move { .. }
                        | Order::Capture
                )
        };
        let bodies: Vec<(u32, Pos, bool)> = self
            .entities
            .iter()
            .filter(|entity| grounded(entity))
            .map(|entity| (entity.id, entity.pos, standing(entity)))
            .collect();
        let mut movers: Vec<u32> = bodies
            .iter()
            .filter(|(id, pos, still)| {
                *still
                    && bodies.iter().any(|(other, other_pos, other_still)| {
                        other != id
                            && pos.distance_sq(*other_pos) < near
                            && (!*other_still || other < id)
                    })
            })
            .map(|(id, _, _)| *id)
            .collect();
        movers.sort_unstable();
        let mut claimed: Vec<(i32, i32)> = Vec::new();
        let gate = self.map.gate_pos;
        let station = i64::from(FP * 2).pow(2);
        for id in movers {
            let Some(entity) = self.entity(id).cloned() else {
                continue;
            };
            let (cx, cy) = entity.pos.cell_xy();
            let mut best: Option<(i64, i32, i32)> = None;
            for (dx, dy) in [
                (0, 0),
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
            ] {
                let cell = (cx + dx, cy + dy);
                let centre = Pos::cell(cell.0, cell.1);
                if !self.cell_walkable_for(entity.kind, cell)
                    || claimed.contains(&cell)
                    || (matches!(entity.order, Order::Capture)
                        && centre.distance_sq(gate) > station)
                    || bodies.iter().any(|(other, other_pos, _)| {
                        *other != id && other_pos.distance_sq(centre) < near
                    })
                {
                    continue;
                }
                let distance = entity.pos.distance_sq(centre);
                if best.is_none_or(|(d, y, x)| (distance, cell.1, cell.0) < (d, y, x)) {
                    best = Some((distance, cell.1, cell.0));
                }
            }
            let Some((_, y, x)) = best else {
                continue;
            };
            claimed.push((x, y));
            let speed = self.speed_per_tick(&entity).max(1);
            let (new_pos, _) = move_towards(entity.pos, Pos::cell(x, y), speed);
            if let Some(entity) = self.entity_mut(id) {
                entity.pos = new_pos;
            }
        }
    }

    pub(crate) fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        kind: Kind,
    ) -> Vec<(i32, i32)> {
        self.search_path(start, goal, kind, &[], NAV_EXPANSION_LIMIT)
    }

    /// The bounded A* behind `find_path`; `avoid` lists cells a machine
    /// may not stand on for this one search (machines standing still).
    fn search_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        kind: Kind,
        avoid: &[(i32, i32)],
        expansion_limit: usize,
    ) -> Vec<(i32, i32)> {
        let footprint = spec(kind).footprint.max(1);
        let avoided = |cell: (i32, i32)| {
            !avoid.is_empty()
                && (0..footprint)
                    .any(|fy| (0..footprint).any(|fx| avoid.contains(&(cell.0 + fx, cell.1 + fy))))
        };
        if start == goal {
            return Vec::new();
        }
        if !self.in_bounds(start) || !self.in_bounds(goal) {
            return Vec::new();
        }
        // A machine already in deep tidal water may wade out of it: the
        // tide's own cells are open to it until it reaches ground.
        let swamped = self.deep_tidal(start) && !matches!(spec(kind).movement, Movement::Hull);
        // The buildings' cells, read once rather than from every entity at
        // every cell the search looks at.
        let blocked = self.blocked_cells();
        let enterable = |cell: (i32, i32), swamped: bool| {
            self.cell_enterable_with(kind, cell, swamped, |covered| {
                self.cell_index(covered).is_some_and(|index| blocked[index])
            })
        };
        if !enterable(start, swamped) || !enterable(goal, false) {
            return Vec::new();
        }

        let width = i32::from(self.map.width);
        let area = usize::from(self.map.width).saturating_mul(usize::from(self.map.height));
        let start_index = self.cell_index(start).expect("start was bounds checked");
        let goal_index = self.cell_index(goal).expect("goal was bounds checked");
        let mut open = BinaryHeap::new();
        let mut costs = vec![i32::MAX; area];
        let mut parents = vec![-1i32; area];
        costs[start_index] = 0;
        open.push(NavNode {
            f: heuristic(start, goal),
            g: 0,
            x: start.0,
            y: start.1,
        });

        let mut expansions = 0usize;
        while let Some(node) = open.pop() {
            let index = (node.y * width + node.x) as usize;
            if node.g != costs[index] {
                continue;
            }
            if index == goal_index {
                let mut result = Vec::new();
                let mut current = index as i32;
                while current as usize != start_index {
                    let x = current % width;
                    let y = current / width;
                    result.push((x, y));
                    let parent = parents[current as usize];
                    if parent < 0 {
                        return Vec::new();
                    }
                    current = parent;
                    if result.len() >= MAX_PATH {
                        return Vec::new();
                    }
                }
                result.reverse();
                return result;
            }
            expansions = expansions.saturating_add(1);
            if expansions >= expansion_limit {
                break;
            }
            for (dx, dy) in NAV_NEIGHBORS {
                let next = (node.x + dx, node.y + dy);
                if !self.in_bounds(next) || !enterable(next, swamped) || avoided(next) {
                    continue;
                }
                let diagonal = dx != 0 && dy != 0;
                if diagonal
                    && (!enterable((node.x + dx, node.y), swamped)
                        || !enterable((node.x, node.y + dy), swamped))
                {
                    continue;
                }
                let next_index = self.cell_index(next).expect("neighbor was bounds checked");
                let step_cost = self.path_cost(next, kind);
                let step_cost = if diagonal {
                    step_cost.saturating_mul(NAV_DIAGONAL_COST) / 100
                } else {
                    step_cost
                };
                let next_cost = node.g.saturating_add(step_cost);
                if next_cost < costs[next_index] {
                    costs[next_index] = next_cost;
                    parents[next_index] = index as i32;
                    open.push(NavNode {
                        f: next_cost.saturating_add(heuristic(next, goal)),
                        g: next_cost,
                        x: next.0,
                        y: next.1,
                    });
                }
            }
        }
        Vec::new()
    }

    pub(crate) fn path_cost(&self, cell: (i32, i32), kind: Kind) -> i32 {
        let movement = spec(kind).movement;
        if matches!(movement, Movement::Air) {
            return 100;
        }
        let terrain = self.map.terrain(cell.0, cell.1);
        match self.depth_at(cell.0, cell.1) {
            Some(Depth::Dry) => 100,
            Some(Depth::Shallow) => {
                if matches!(movement, Movement::Hull) {
                    100
                } else {
                    lane_cost(false, movement)
                }
            }
            Some(Depth::Deep) => {
                if matches!(movement, Movement::Hull) || kind == Kind::Dredger {
                    100
                } else {
                    300
                }
            }
            None => match terrain {
                Terrain::Silt => 110,
                _ => 100,
            },
        }
    }

    pub(crate) fn speed_per_tick(&self, entity: &Entity) -> i32 {
        let movement = spec(entity.kind).movement;
        let tracks = self.players[entity.owner as usize]
            .upgrades
            .contains(&Upgrade::Tracks);
        // A Dredger takes shallow water at full speed and deep tidal water
        // nearly so; Tracks halve the shallow slow; a swamped machine crawls.
        let (cell_x, cell_y) = entity.pos.cell_xy();
        let terrain = self.map.terrain(cell_x, cell_y);
        let percent = if matches!(movement, Movement::Air | Movement::Hull) {
            100
        } else {
            match self.depth_at(cell_x, cell_y) {
                Some(Depth::Dry) => 100,
                Some(Depth::Shallow) => {
                    if entity.kind == Kind::Dredger {
                        100
                    } else {
                        let base = lane_speed(false, movement);
                        if tracks { 100 - (100 - base) / 2 } else { base }
                    }
                }
                Some(Depth::Deep) => {
                    if entity.kind == Kind::Dredger {
                        90
                    } else {
                        bw_content::SWAMPED_SPEED_PERCENT
                    }
                }
                None => match terrain {
                    Terrain::Silt => 95,
                    _ => 100,
                },
            }
        };
        let surge = entity.surge_remaining > 0;
        let boost_numerator = if surge { 3i64 } else { 1i64 };
        let boost_denominator = if surge { 2i64 } else { 1i64 };
        let base = if entity.pace > 0 {
            spec(entity.kind).speed.min(entity.pace)
        } else {
            spec(entity.kind).speed
        };
        let numerator = i64::from(base)
            .saturating_mul(i64::from(percent))
            .saturating_mul(boost_numerator);
        // Distribute fractional raw-coordinate steps on the deterministic
        // tick phase instead of truncating the same remainder every tick.
        let denominator = TICK_HZ as i64 * 100 * boost_denominator;
        let phase = (self.tick % denominator as u64) as i64;
        let speed = (phase + 1) * numerator / denominator - phase * numerator / denominator;
        speed.clamp(0, i64::from(i32::MAX)) as i32
    }

    pub(crate) fn occupied_by_other(&self, id: u32, cell: (i32, i32)) -> bool {
        // A machine aboard a transport is not on the field; flight and
        // ground never block each other.
        let mover_flies = self
            .entity(id)
            .is_some_and(|entity| matches!(spec(entity.kind).movement, Movement::Air));
        self.entities.iter().any(|entity| {
            entity.id != id
                && entity.hp > 0
                && entity.aboard.is_none()
                && matches!(spec(entity.kind).movement, Movement::Air) == mover_flies
                && !self.soft_gather_collision(id, entity.id)
                && footprint_contains(entity, cell)
        })
    }

    /// The nearest free cell a machine of `kind` may stand on: a hull
    /// looks for water, a lifter for any cell, the rest for ground.
    pub(crate) fn find_spawn_pos_for(&self, kind: Kind, origin: Pos) -> Option<Pos> {
        let (x, y) = origin.cell_xy();
        for radius in 1..=SPAWN_SEARCH_RADIUS {
            for (dx, dy) in ring_offsets(radius) {
                let cell = (x + dx, y + dy);
                if self.in_bounds(cell)
                    && self.cell_walkable_for(kind, cell)
                    && !self.occupied_by_other(u32::MAX, cell)
                {
                    return Some(Pos::cell(cell.0, cell.1));
                }
            }
        }
        None
    }

    pub(crate) fn line_of_sight(&self, from: Pos, to: Pos, ignored: Option<u32>) -> bool {
        let start = from.cell_xy();
        let end = to.cell_xy();
        let endpoint_building = self
            .entities
            .iter()
            .find(|entity| entity.kind.is_building() && footprint_contains(entity, end))
            .map(|entity| entity.id);
        for (cell, is_endpoint) in grid_line(start, end) {
            if self.map.terrain(cell.0, cell.1) == Terrain::Rock {
                return false;
            }
            if is_endpoint || cell == start {
                continue;
            }
            if self.entities.iter().any(|entity| {
                Some(entity.id) != ignored
                    && Some(entity.id) != endpoint_building
                    && entity.hp > 0
                    && entity.kind.is_building()
                    && footprint_contains(entity, cell)
            }) {
                return false;
            }
        }
        true
    }

    pub(crate) fn nearest_dropoff(&self, player: u8, pos: Pos) -> Option<Pos> {
        let worker = self.entities.iter().find(|entity| {
            entity.owner == player
                && entity.hp > 0
                && entity.kind.is_worker()
                && entity.carried > 0
                && entity.pos == pos
        });
        if let Some(worker) = worker
            && let Some(target) = worker.path_target
            && self.entities.iter().any(|building| {
                building.owner == player
                    && building.hp > 0
                    && building.build_remaining == 0
                    && matches!(building.kind, Kind::Headquarters | Kind::Dropoff)
                    && self.is_interaction_slot(building, target)
            })
        {
            // A hauling worker keeps its assigned perimeter slot while
            // moving. Re-selecting the nearest free slot every tick made the
            // target jump as traffic moved around the yard.
            return Some(target);
        }

        let worker_id = worker.map(|worker| worker.id).unwrap_or(0);
        self.entities
            .iter()
            .filter(|entity| {
                entity.owner == player
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && matches!(entity.kind, Kind::Headquarters | Kind::Dropoff)
            })
            .min_by_key(|entity| (entity.pos.distance_sq(pos), entity.id))
            .map(|entity| self.stable_interaction_target(entity, worker_id, pos))
    }

    pub(crate) fn interaction_target(&self, building: &Entity, from: Pos) -> Pos {
        if let Some(target) = self.pinned_interaction_target(building, from) {
            return target;
        }
        let candidates = self
            .interaction_slots(building)
            .into_iter()
            .map(|target| {
                let cell = target.cell_xy();
                (
                    self.interaction_occupied(cell, building.id, from),
                    target.distance_sq(from),
                    cell.1,
                    cell.0,
                    target,
                )
            })
            .collect::<Vec<_>>();
        candidates
            .into_iter()
            .min_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3))
            .map(|candidate| candidate.4)
            .unwrap_or(building.pos)
    }

    fn stable_interaction_target(&self, building: &Entity, worker_id: u32, from: Pos) -> Pos {
        let mut slots = self.interaction_slots(building);
        if slots.is_empty() {
            return building.pos;
        }
        slots.sort_by_key(|target| {
            let cell = target.cell_xy();
            (target.distance_sq(from), cell.1, cell.0)
        });
        slots[(worker_id as usize + building.id as usize) % slots.len()]
    }

    fn pinned_interaction_target(&self, building: &Entity, from: Pos) -> Option<Pos> {
        self.entities
            .iter()
            .find(|entity| {
                entity.owner == building.owner
                    && entity.hp > 0
                    && entity.kind.is_worker()
                    && entity.pos == from
                    && matches!(
                        entity.order,
                        Order::Build { target } | Order::Repair { target } if target == building.id
                    )
            })
            .and_then(|worker| worker.path_target)
            .filter(|target| self.is_interaction_slot(building, *target))
    }

    fn interaction_slots(&self, building: &Entity) -> Vec<Pos> {
        let (x, y) = building.pos.cell_xy();
        let footprint = spec(building.kind).footprint.max(1);
        let mut perimeter = Vec::with_capacity((footprint + 1) as usize * 4);
        for px in -1..=footprint {
            perimeter.push((px, -1));
            perimeter.push((px, footprint));
        }
        for py in 0..footprint {
            perimeter.push((-1, py));
            perimeter.push((footprint, py));
        }
        perimeter
            .into_iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .filter(|cell| {
                self.in_bounds(*cell)
                    && self.map.terrain(cell.0, cell.1).walkable()
                    && !self.building_occupies(*cell, Some(building.id))
            })
            .map(|cell| Pos::cell(cell.0, cell.1))
            .collect()
    }

    fn is_interaction_slot(&self, building: &Entity, target: Pos) -> bool {
        self.interaction_slots(building)
            .into_iter()
            .any(|slot| slot == target)
    }

    pub(crate) fn resource_work_target(&self, resource: &Resource, worker_id: u32) -> Pos {
        const OFFSETS: [(i32, i32); 8] = [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
        ];
        let (x, y) = resource.pos.cell_xy();
        for index in 0..OFFSETS.len() {
            let offset = OFFSETS[(worker_id as usize + index) % OFFSETS.len()];
            let cell = (x + offset.0, y + offset.1);
            if self.in_bounds(cell)
                && self.map.terrain(cell.0, cell.1).walkable()
                && !self.building_occupies(cell, None)
            {
                return Pos::cell(cell.0, cell.1);
            }
        }
        resource.pos
    }

    pub(crate) fn can_place(&self, kind: Kind, pos: Pos) -> bool {
        self.can_place_for(None, kind, pos)
    }

    /// Whether `player` may put a site here.  Its own machines on their
    /// feet do not refuse it: they step off when the site goes down
    /// (rules 14).  Buildings, deployed machines and the enemy on the
    /// ground still do; nothing in the air does.
    pub(crate) fn can_place_for(&self, player: Option<u8>, kind: Kind, pos: Pos) -> bool {
        let (x, y) = pos.cell_xy();
        let footprint = spec(kind).footprint.max(1);
        for fy in 0..footprint {
            for fx in 0..footprint {
                let cell = (x + fx, y + fy);
                if !self.in_bounds(cell)
                    || !matches!(
                        self.map.terrain(cell.0, cell.1),
                        Terrain::Salt | Terrain::Silt
                    )
                    || Pos::cell(cell.0, cell.1).distance_sq(self.map.gate_pos)
                        <= i64::from(FP * 4).pow(2)
                {
                    return false;
                }
                if self.entities.iter().any(|entity| {
                    entity.hp > 0
                        && entity.aboard.is_none()
                        && !matches!(spec(entity.kind).movement, Movement::Air)
                        && !(Some(entity.owner) == player && steps_aside(entity))
                        && footprint_overlaps_rect(entity, (x, y), footprint)
                }) {
                    return false;
                }
            }
        }
        kind != Kind::Condenser || self.map.wells.iter().any(|well| well.cell_xy() == (x, y))
    }

    fn navigation_target(&self, entity: &Entity, reach: &mut Reach) -> Option<(Pos, StopRule)> {
        match entity.order {
            Order::Move { target } | Order::AttackMove { target } => {
                Some((target, StopRule::Exact { target }))
            }
            Order::Gather { resource } => {
                if entity.carried > 0 {
                    self.nearest_dropoff(entity.owner, entity.pos)
                        .map(|target| (target, StopRule::Interaction))
                } else {
                    self.map
                        .resources
                        .iter()
                        .find(|item| item.id == resource)
                        .map(|item| {
                            (
                                self.resource_work_target(item, entity.id),
                                StopRule::Interaction,
                            )
                        })
                }
            }
            Order::Build { target }
            | Order::Repair { target }
            | Order::Recycle { target }
            | Order::Deliver { target, .. } => self.entity(target).map(|building| {
                (
                    self.interaction_target(building, entity.pos),
                    StopRule::Interaction,
                )
            }),
            Order::Attack { target } => {
                let target_entity = self.entity(target)?;
                if target_entity.owner == entity.owner
                    || target_entity.hp <= 0
                    || !self.entity_visible(entity.owner, target)
                {
                    return None;
                }
                let range = self.weapon_range(entity);
                let min_range = if entity.kind == Kind::Loom {
                    LOOM_MIN_RANGE
                } else {
                    0
                };
                if entity.pos.distance_sq(target_entity.pos) <= i64::from(range).pow(2)
                    && entity.pos.distance_sq(target_entity.pos) >= i64::from(min_range).pow(2)
                    && self.line_of_sight(entity.pos, target_entity.pos, Some(entity.id))
                {
                    return None;
                }
                self.attack_goal(entity, target_entity, range, min_range, reach)
                    .map(|goal| {
                        (
                            goal,
                            StopRule::Weapon {
                                target: target_entity.pos,
                                range,
                                min_range,
                            },
                        )
                    })
            }
            Order::Capture => self.capture_goal(entity).map(|goal| (goal, StopRule::Gate)),
            Order::Board { transport } => {
                let carrier = self.entity(transport)?;
                if carrier.hp <= 0 || carrier.owner != entity.owner {
                    return None;
                }
                Some((carrier.pos, StopRule::Board { transport }))
            }
            Order::Lay { head, .. } => Some((head, StopRule::Exact { target: head })),
            _ => None,
        }
    }

    fn stop_reached(&self, entity: &Entity, rule: StopRule) -> bool {
        match rule {
            StopRule::Exact { target } => entity.pos == target,
            StopRule::Weapon {
                target,
                range,
                min_range,
            } => {
                entity.pos.distance_sq(target) <= i64::from(range).pow(2)
                    && entity.pos.distance_sq(target) >= i64::from(min_range).pow(2)
                    && self.line_of_sight(entity.pos, target, Some(entity.id))
            }
            StopRule::Gate => entity.pos.distance_sq(self.map.gate_pos) <= i64::from(FP * 2).pow(2),
            StopRule::Interaction => false,
            StopRule::Board { transport } => self.entity(transport).is_some_and(|carrier| {
                entity.pos.distance_sq(carrier.pos)
                    <= i64::from(FP * bw_content::BOARD_RADIUS_CELLS).pow(2)
            }),
        }
    }

    fn attack_goal(
        &self,
        entity: &Entity,
        target: &Entity,
        range: i32,
        min_range: i32,
        reach: &mut Reach,
    ) -> Option<Pos> {
        let radius = (range / FP).clamp(1, NAV_MAX_DESTINATION_RADIUS * 2);
        let target_cell = target.pos.cell_xy();
        let mut candidates = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let cell = (target_cell.0 + dx, target_cell.1 + dy);
                if !self.in_bounds(cell) || !self.cell_walkable_for(entity.kind, cell) {
                    continue;
                }
                let candidate = Pos::cell(cell.0, cell.1);
                if candidate.distance_sq(target.pos) <= i64::from(range).pow(2)
                    && candidate.distance_sq(target.pos) >= i64::from(min_range).pow(2)
                {
                    candidates.push((
                        candidate.distance_sq(entity.pos),
                        candidate.distance_sq(target.pos),
                        cell.1,
                        cell.0,
                        candidate,
                    ));
                }
            }
        }
        candidates.sort_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3));
        // A target across water or behind a wall has every candidate out of
        // reach, and each failed search ran to its step limit: 27 machines
        // sent at one in trial 10 slowed the match forty-fold. Candidates
        // outside the area the machine can reach are skipped, since their
        // searches could only fail; the area is found after the first
        // failure and shared by every machine that starts inside it.
        let start = entity.pos.cell_xy();
        let mut area = reach.area_of(self, start, entity.kind);
        candidates.into_iter().find_map(|(_, _, _, _, candidate)| {
            if !self.line_of_sight(candidate, target.pos, Some(entity.id)) {
                return None;
            }
            let cell = candidate.cell_xy();
            if let (Some(area), Some(index)) = (area, self.cell_index(cell))
                && !reach.areas[area].1[index]
            {
                return None;
            }
            if !self.find_path(start, cell, entity.kind).is_empty() {
                return Some(candidate);
            }
            if area.is_none() {
                area = Some(reach.find(self, start, entity.kind));
            }
            None
        })
    }

    /// Where a capturing machine outside the ring walks: the nearest cell
    /// inside the ring that no other machine stands on, or the nearest
    /// cell inside it when every one is taken.  Each capturer picks its own
    /// cell inside the ring; capture orders are never spread as a formation
    /// (rules 21, trial 11-a), since a formation spot can lie outside the
    /// ring, where the machine stood and never channelled.
    fn capture_goal(&self, entity: &Entity) -> Option<Pos> {
        let ring = i64::from(FP * 2).pow(2);
        if entity.pos.distance_sq(self.map.gate_pos) <= ring {
            return None;
        }
        let flies = matches!(spec(entity.kind).movement, Movement::Air);
        // Only standing machines take a cell: one walking through does not
        // turn the others away, so goals do not flicker with traffic.
        let taken = |cell: (i32, i32)| {
            self.entities.iter().any(|other| {
                other.id != entity.id
                    && other.hp > 0
                    && other.aboard.is_none()
                    && matches!(spec(other.kind).movement, Movement::Air) == flies
                    && other.path.is_empty()
                    && footprint_contains(other, cell)
            })
        };
        let (gx, gy) = self.map.gate_pos.cell_xy();
        let mut candidates = Vec::new();
        for dy in -2..=2 {
            for dx in -2..=2 {
                let cell = (gx + dx, gy + dy);
                if self.in_bounds(cell) && self.cell_walkable_for(entity.kind, cell) {
                    let candidate = Pos::cell(cell.0, cell.1);
                    if candidate.distance_sq(self.map.gate_pos) <= ring {
                        candidates.push((
                            taken(cell),
                            candidate.distance_sq(entity.pos),
                            cell.1,
                            cell.0,
                            candidate,
                        ));
                    }
                }
            }
        }
        candidates.sort_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3));
        candidates
            .into_iter()
            .map(|(_, _, _, _, candidate)| candidate)
            .next()
    }

    /// Machines sent to one point stand in a formation around it, in id
    /// order.  Only a move or attack-move to the same cell forms a group;
    /// any order whose goal is worked out per machine (a capture, an
    /// attack, a gather) goes to its own goal.
    fn group_destination(&self, id: u32, requested: Pos, order: &Order, kind: Kind) -> Pos {
        let owner = self.entity(id).map(|entity| entity.owner);
        let rank = self
            .entities
            .iter()
            .filter(|other| {
                other.id < id
                    && Some(other.owner) == owner
                    && same_group_destination(other, requested, order)
            })
            .count();
        let requested = self.resolve_goal(requested, kind);
        if rank == 0 {
            return requested;
        }
        let (x, y) = requested.cell_xy();
        let mut clear_seen = 0usize;
        for radius in 0..=NAV_MAX_DESTINATION_RADIUS {
            for (dx, dy) in ring_offsets(radius) {
                let cell = (x + dx, y + dy);
                if !self.in_bounds(cell) || !self.cell_walkable_for(kind, cell) {
                    continue;
                }
                if clear_seen == rank {
                    return Pos::cell(cell.0, cell.1);
                }
                clear_seen = clear_seen.saturating_add(1);
            }
        }
        requested
    }

    /// A machine held up by others that are standing still (idle, dug in,
    /// working, or stuck themselves) plans a way around them. Without it a
    /// machine behind a standing one stepped aside, stepped back and was
    /// held up again for the rest of the match.
    fn assign_way_around(
        &mut self,
        id: u32,
        blocked: (i32, i32),
        goal: Pos,
        kind: Kind,
        age: u32,
    ) -> bool {
        if matches!(spec(kind).movement, Movement::Air) {
            return false;
        }
        let Some(entity) = self.entity(id).cloned() else {
            return false;
        };
        let start = entity.pos.cell_xy();
        let mut standing: Vec<(i32, i32)> = Vec::new();
        for other in &self.entities {
            if other.id == id
                || other.hp <= 0
                || other.aboard.is_some()
                || other.kind.is_building()
                || matches!(spec(other.kind).movement, Movement::Air)
                || self.soft_gather_collision(id, other.id)
                || (!other.path.is_empty() && other.blocked_ticks == 0)
            {
                continue;
            }
            let (x, y) = other.pos.cell_xy();
            if (x - start.0).abs().max((y - start.1).abs()) > NAV_WAY_AROUND_RADIUS {
                continue;
            }
            let size = spec(other.kind).footprint.max(1);
            for fy in 0..size {
                for fx in 0..size {
                    standing.push((x + fx, y + fy));
                }
            }
        }
        if !standing.contains(&blocked) {
            return false;
        }
        // A machine standing on the goal itself is waited for, not avoided:
        // an order to board or stand beside it must still find a path.
        let effective_goal = self.resolve_goal(goal, kind);
        standing.retain(|cell| *cell != effective_goal.cell_xy());
        let cells = self.search_path(
            start,
            effective_goal.cell_xy(),
            kind,
            &standing,
            NAV_WAY_AROUND_EXPANSIONS,
        );
        if cells.first().is_none_or(|first| *first == blocked) {
            return false;
        }
        let mut path: Vec<Pos> = cells.into_iter().map(|(x, y)| Pos::cell(x, y)).collect();
        if path.last().is_some_and(|last| *last != effective_goal) {
            path.push(effective_goal);
        }
        let lane_revision = self.gate.lane_revision;
        if let Some(entity) = self.entity_mut(id) {
            entity.path = path;
            entity.path_index = 0;
            entity.path_target = Some(effective_goal);
            entity.path_lane_revision = lane_revision;
            entity.blocked_ticks = age.saturating_add(1);
        }
        true
    }

    fn assign_detour(
        &mut self,
        id: u32,
        blocked: (i32, i32),
        goal: Pos,
        kind: Kind,
        age: u32,
    ) -> bool {
        let current_lane_revision = self.gate.lane_revision;
        let Some(entity) = self.entity(id).cloned() else {
            return false;
        };
        let origin = entity.pos.cell_xy();
        let phase = (age / NAV_DETOUR_AFTER_BLOCKED) as usize;
        for step in 0..NAV_DETOUR_NEIGHBORS.len() {
            let (dx, dy) =
                NAV_DETOUR_NEIGHBORS[(phase + id as usize + step) % NAV_DETOUR_NEIGHBORS.len()];
            let cell = (origin.0 + dx, origin.1 + dy);
            if cell == blocked
                || !self.in_bounds(cell)
                || !self.cell_walkable_for(kind, cell)
                || self.occupied_by_other(id, cell)
            {
                continue;
            }
            if let Some(entity) = self.entity_mut(id) {
                entity.path = vec![Pos::cell(cell.0, cell.1)];
                entity.path_index = 0;
                entity.path_target = Some(goal);
                entity.path_lane_revision = current_lane_revision;
                entity.blocked_ticks = age.saturating_add(1);
            }
            return true;
        }
        false
    }

    fn clear_navigation_path(&mut self, id: u32) {
        if let Some(entity) = self.entity_mut(id) {
            entity.path.clear();
            entity.path_index = 0;
            entity.path_target = None;
            entity.blocked_ticks = 0;
        }
    }

    fn in_bounds(&self, cell: (i32, i32)) -> bool {
        cell.0 >= 0
            && cell.1 >= 0
            && cell.0 < i32::from(self.map.width)
            && cell.1 < i32::from(self.map.height)
    }

    fn cell_index(&self, cell: (i32, i32)) -> Option<usize> {
        self.in_bounds(cell)
            .then_some(cell.1 as usize * self.map.width as usize + cell.0 as usize)
    }

    fn resolve_goal(&self, requested: Pos, kind: Kind) -> Pos {
        let cell = requested.cell_xy();
        if self.cell_walkable_for(kind, cell) {
            return requested;
        }
        for radius in 1..=NAV_GOAL_RESOLUTION_RADIUS {
            let mut candidates = ring_offsets(radius);
            candidates.sort_unstable_by_key(|candidate| {
                let point = Pos::cell(cell.0 + candidate.0, cell.1 + candidate.1);
                (point.distance_sq(requested), candidate.1, candidate.0)
            });
            for (dx, dy) in candidates {
                let candidate = (cell.0 + dx, cell.1 + dy);
                if self.cell_walkable_for(kind, candidate) {
                    return Pos::cell(candidate.0, candidate.1);
                }
            }
        }
        requested
    }

    /// Whether any of `units` could walk to `to` as the ground lies now.
    /// Read-only and never part of a step: the interface asks it so an
    /// order no machine could carry out says why instead of doing nothing
    /// (trial 10: an attack-move across a deep lane).  A machine aboard,
    /// in the air, or standing where no search starts counts as able.
    pub fn any_can_reach(&self, units: &[u32], to: Pos) -> bool {
        let mut reach = Reach::default();
        for id in units {
            let Some(entity) = self.entity(*id) else {
                continue;
            };
            if entity.aboard.is_some() || matches!(spec(entity.kind).movement, Movement::Air) {
                return true;
            }
            let start = entity.pos.cell_xy();
            let goal = self.resolve_goal(to, entity.kind).cell_xy();
            let area = reach.find(self, start, entity.kind);
            let seen = &reach.areas[area].1;
            let at = |cell| self.cell_index(cell).is_some_and(|index| seen[index]);
            if !at(start) || at(goal) {
                return true;
            }
        }
        false
    }

    /// Whether any of `units` stands, or could walk to a cell, within
    /// `within(unit)` of `to`: an attacker to its weapon's reach, a worker
    /// beside a wreck, a capturer into the station's ring.  Read-only and
    /// never part of a step, like `any_can_reach`: the interface asks it
    /// so an order no machine could carry out is refused at once (trial
    /// 12: attack, gather and capture orders across a deep lane were
    /// accepted and did nothing).
    pub fn any_can_reach_within(
        &self,
        units: &[u32],
        to: Pos,
        within: impl Fn(&Entity) -> i32,
    ) -> bool {
        let mut reach = Reach::default();
        for id in units {
            let Some(entity) = self.entity(*id) else {
                continue;
            };
            if entity.aboard.is_some() || matches!(spec(entity.kind).movement, Movement::Air) {
                return true;
            }
            let range = i64::from(within(entity).max(0));
            if entity.pos.distance_sq(to) <= range.pow(2) {
                return true;
            }
            let start = entity.pos.cell_xy();
            let goal = self.resolve_goal(to, entity.kind).cell_xy();
            let area = reach.find(self, start, entity.kind);
            let seen = &reach.areas[area].1;
            let at = |cell| self.cell_index(cell).is_some_and(|index| seen[index]);
            if !at(start) || at(goal) {
                return true;
            }
            let radius = (range / i64::from(FP)) as i32 + 1;
            let (tx, ty) = to.cell_xy();
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let cell = (tx + dx, ty + dy);
                    if at(cell) && Pos::cell(cell.0, cell.1).distance_sq(to) <= range.pow(2) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn cell_walkable_for(&self, kind: Kind, cell: (i32, i32)) -> bool {
        self.cell_enterable(kind, cell, false)
    }

    /// Whether a machine of `kind` may stand on `cell`: ground movement
    /// takes dry and shallow tidal cells and deep ones only while swamped
    /// (or as a Dredger); a hull takes water of any depth and never dry
    /// ground; flight takes everything but rock and ignores blockers.
    pub(crate) fn cell_enterable(&self, kind: Kind, cell: (i32, i32), swamped: bool) -> bool {
        self.cell_enterable_with(kind, cell, swamped, |covered| {
            self.building_occupies(covered, None)
        })
    }

    /// `cell_enterable` with the building test given: a search asks it
    /// hundreds of thousands of times and reads it from `blocked_cells`.
    fn cell_enterable_with(
        &self,
        kind: Kind,
        cell: (i32, i32),
        swamped: bool,
        occupied: impl Fn((i32, i32)) -> bool,
    ) -> bool {
        if !self.in_bounds(cell) {
            return false;
        }
        let movement = spec(kind).movement;
        if matches!(movement, Movement::Air) {
            return self.map.terrain(cell.0, cell.1) != Terrain::Rock;
        }
        let footprint = spec(kind).footprint.max(1);
        for fy in 0..footprint {
            for fx in 0..footprint {
                let covered = (cell.0 + fx, cell.1 + fy);
                if !self.in_bounds(covered) {
                    return false;
                }
                let terrain = self.map.terrain(covered.0, covered.1);
                let ok = match (movement, self.depth_at(covered.0, covered.1)) {
                    (Movement::Hull, Some(Depth::Dry)) => false,
                    (Movement::Hull, Some(_)) => true,
                    (Movement::Hull, None) => terrain == Terrain::Deep,
                    (_, Some(Depth::Deep)) => swamped || kind == Kind::Dredger,
                    (_, Some(_)) => true,
                    (_, None) => terrain.walkable(),
                };
                if !ok {
                    return false;
                }
                if occupied(covered) {
                    return false;
                }
            }
        }
        true
    }

    /// Whether `cell` is tidal water at deep tide now.
    pub(crate) fn deep_tidal(&self, cell: (i32, i32)) -> bool {
        self.depth_at(cell.0, cell.1) == Some(Depth::Deep)
    }

    /// Buildings and deployed Caissons are fixed blockers.
    fn building_occupies(&self, cell: (i32, i32), ignored: Option<u32>) -> bool {
        self.entities.iter().any(|entity| {
            entity.hp > 0
                && (entity.kind.is_building() || (entity.kind == Kind::Caisson && entity.deployed))
                && Some(entity.id) != ignored
                && footprint_contains(entity, cell)
        })
    }

    /// `building_occupies(cell, None)` for every cell of the map at once,
    /// by cell index.
    fn blocked_cells(&self) -> Vec<bool> {
        let area = usize::from(self.map.width) * usize::from(self.map.height);
        let mut blocked = vec![false; area];
        for entity in &self.entities {
            if entity.hp > 0
                && (entity.kind.is_building() || (entity.kind == Kind::Caisson && entity.deployed))
            {
                let (x, y) = entity.pos.cell_xy();
                let size = spec(entity.kind).footprint.max(1);
                for cy in y..y + size {
                    for cx in x..x + size {
                        if let Some(index) = self.cell_index((cx, cy)) {
                            blocked[index] = true;
                        }
                    }
                }
            }
        }
        blocked
    }

    /// Every cell a machine of `kind` could walk to from `start` by the
    /// moves `search_path` takes, with no step limit, by cell index. A goal
    /// outside it is one no search can reach, so its search need not run.
    fn reachable_cells(&self, start: (i32, i32), kind: Kind, blocked: &[bool]) -> Vec<bool> {
        let area = usize::from(self.map.width) * usize::from(self.map.height);
        let mut seen = vec![false; area];
        let swamped = self.deep_tidal(start) && !matches!(spec(kind).movement, Movement::Hull);
        let enterable = |cell: (i32, i32)| {
            self.cell_enterable_with(kind, cell, swamped, |covered| {
                self.cell_index(covered).is_some_and(|index| blocked[index])
            })
        };
        let Some(start_index) = self.cell_index(start) else {
            return seen;
        };
        if !enterable(start) {
            return seen;
        }
        seen[start_index] = true;
        let mut stack = vec![start];
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in NAV_NEIGHBORS {
                let next = (x + dx, y + dy);
                let Some(index) = self.cell_index(next) else {
                    continue;
                };
                if seen[index] || !enterable(next) {
                    continue;
                }
                if dx != 0 && dy != 0 && (!enterable((x + dx, y)) || !enterable((x, y + dy))) {
                    continue;
                }
                seen[index] = true;
                stack.push(next);
            }
        }
        seen
    }

    fn interaction_occupied(&self, cell: (i32, i32), building_id: u32, from: Pos) -> bool {
        self.entities.iter().any(|entity| {
            entity.id != building_id
                && entity.hp > 0
                && entity.pos != from
                && footprint_contains(entity, cell)
        })
    }

    /// Two own machines that may share a cell: gatherers working the same
    /// wreck, and anything passing an own deployed specialist.  A deployed
    /// line penned its own reinforcements in the seventh trial, with no way
    /// out and no message.
    fn soft_gather_collision(&self, id: u32, other_id: u32) -> bool {
        let Some(entity) = self.entity(id) else {
            return false;
        };
        let Some(other) = self.entity(other_id) else {
            return false;
        };
        if entity.owner != other.owner {
            return false;
        }
        if other.deployed && crate::is_specialist(other.kind) && !entity.kind.is_building() {
            return true;
        }
        entity.kind.is_worker()
            && other.kind.is_worker()
            && matches!(entity.order, Order::Gather { .. })
            && matches!(other.order, Order::Gather { .. })
    }
}

/// Whether `entity` belongs to the formation of a machine with `order` to
/// `requested`, the target as ordered (before it is moved off water or
/// rock, so a group sent onto water still spreads).  Only moves and
/// attack-moves to one cell group.  A capture never does (rules 21): in
/// trial 11-a every capture order on the map made one formation, led by a
/// Glazier far away, and six machines beside the station were given spots
/// outside the ring where capturing counts.
fn same_group_destination(entity: &Entity, requested: Pos, order: &Order) -> bool {
    if entity.hp <= 0 || entity.build_remaining > 0 || entity.kind.is_building() {
        return false;
    }
    match (order, &entity.order) {
        (Order::Move { .. }, Order::Move { target })
        | (Order::AttackMove { .. }, Order::AttackMove { target }) => {
            target.cell_xy() == requested.cell_xy()
        }
        _ => false,
    }
}

fn footprint_contains(entity: &Entity, cell: (i32, i32)) -> bool {
    let (x, y) = entity.pos.cell_xy();
    let size = spec(entity.kind).footprint.max(1);
    cell.0 >= x && cell.0 < x + size && cell.1 >= y && cell.1 < y + size
}

/// An own machine that steps off a new site: anything on its feet that is
/// not a building, not deployed and not in the air.
pub(crate) fn steps_aside(entity: &Entity) -> bool {
    !entity.kind.is_building()
        && !entity.deployed
        && entity.deploy_remaining == 0
        && !matches!(spec(entity.kind).movement, Movement::Air)
}

fn footprint_overlaps_rect(entity: &Entity, origin: (i32, i32), size: i32) -> bool {
    let (x, y) = entity.pos.cell_xy();
    let other = spec(entity.kind).footprint.max(1);
    x < origin.0 + size && origin.0 < x + other && y < origin.1 + size && origin.1 < y + other
}

/// One tick of travel toward `target`, `speed` long in any direction
/// (rules 16): a corner step is not 41% faster than a side step. The
/// distance is an integer square root, so every seat agrees on it.
fn move_towards(current: Pos, target: Pos, speed: i32) -> (Pos, bool) {
    let dx = i64::from(target.x) - i64::from(current.x);
    let dy = i64::from(target.y) - i64::from(current.y);
    let distance = (dx.unsigned_abs().pow(2) + dy.unsigned_abs().pow(2)).isqrt() as i64;
    let speed = i64::from(speed.max(1));
    if distance <= speed {
        return (target, true);
    }
    // Rounded to the nearest unit, not toward zero: truncating both axes of
    // a corner step would lose a unit a tick and walk corners slow.
    let share = |delta: i64| {
        let scaled = delta.saturating_mul(speed);
        (scaled + scaled.signum() * (distance / 2)) / distance
    };
    let step_x = share(dx);
    let step_y = share(dy);
    (
        Pos::raw(
            current.x.saturating_add(step_x as i32),
            current.y.saturating_add(step_y as i32),
        ),
        false,
    )
}

/// Octile distance at the cheapest step cost: exact on open dry ground and
/// never above the true cost, so the search stays optimal.
fn heuristic(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = a.0.saturating_sub(b.0).abs();
    let dy = a.1.saturating_sub(b.1).abs();
    let (long, short) = (dx.max(dy), dx.min(dy));
    long.saturating_mul(100)
        .saturating_add(short.saturating_mul(NAV_DIAGONAL_COST - 100))
}

fn lane_cost(dry: bool, movement: Movement) -> i32 {
    if dry {
        return 100;
    }
    match movement {
        Movement::Wheel => 182,
        Movement::Walker => 133,
        Movement::Paddle => 111,
        Movement::Hull | Movement::Air => 100,
        Movement::Static => 1000,
    }
}

fn lane_speed(dry: bool, movement: Movement) -> i32 {
    if dry {
        return 100;
    }
    match movement {
        Movement::Wheel => 55,
        Movement::Walker => 75,
        Movement::Paddle => 90,
        Movement::Hull | Movement::Air => 100,
        Movement::Static => 0,
    }
}

fn direction_octant(dx: i32, dy: i32) -> u8 {
    match (dx.signum(), dy.signum()) {
        (0, -1) => 0,
        (1, -1) => 1,
        (1, 0) => 2,
        (1, 1) => 3,
        (0, 1) => 4,
        (-1, 1) => 5,
        (-1, 0) => 6,
        (-1, -1) => 7,
        _ => 0,
    }
}

fn ring_offsets(radius: i32) -> Vec<(i32, i32)> {
    if radius == 0 {
        return vec![(0, 0)];
    }
    let mut offsets = Vec::with_capacity((radius * 8) as usize);
    for y in -radius..=radius {
        for x in -radius..=radius {
            if x.abs().max(y.abs()) == radius {
                offsets.push((x, y));
            }
        }
    }
    offsets
}

fn grid_line(start: (i32, i32), end: (i32, i32)) -> Vec<((i32, i32), bool)> {
    let mut points = Vec::new();
    let (mut x, mut y) = start;
    let dx = end.0.saturating_sub(start.0).abs();
    let sx = if start.0 < end.0 { 1 } else { -1 };
    let dy = -end.1.saturating_sub(start.1).abs();
    let sy = if start.1 < end.1 { 1 } else { -1 };
    let mut error = dx.saturating_add(dy);
    let max_points = (dx.saturating_add(-dy)).saturating_add(1).max(1) as usize;
    for _ in 0..max_points {
        points.push(((x, y), (x, y) == end));
        if (x, y) == end {
            break;
        }
        let e2 = error.saturating_mul(2);
        if e2 >= dy {
            error = error.saturating_add(dy);
            x = x.saturating_add(sx);
        }
        if e2 <= dx {
            error = error.saturating_add(dx);
            y = y.saturating_add(sy);
        }
    }
    points
}

/// What decides where a machine may stand: its movement, its footprint,
/// whether it is a Dredger, and whether it starts swamped.
type ReachKey = (Movement, i32, bool, bool);

/// The areas machines can reach, found during one navigation pass and
/// shared by every machine that starts inside one.
#[derive(Default)]
pub(crate) struct Reach {
    blocked: Option<Vec<bool>>,
    areas: Vec<(ReachKey, Vec<bool>)>,
}

impl Reach {
    fn key(world: &World, start: (i32, i32), kind: Kind) -> ReachKey {
        let movement = spec(kind).movement;
        let swamped = world.deep_tidal(start) && !matches!(movement, Movement::Hull);
        (
            movement,
            spec(kind).footprint.max(1),
            kind == Kind::Dredger,
            swamped,
        )
    }

    /// The area already found that holds `start` for this kind of machine.
    fn area_of(&self, world: &World, start: (i32, i32), kind: Kind) -> Option<usize> {
        let index = world.cell_index(start)?;
        let key = Reach::key(world, start, kind);
        self.areas
            .iter()
            .position(|(k, area)| *k == key && area[index])
    }

    /// Find the area `start` lies in, and keep it.
    fn find(&mut self, world: &World, start: (i32, i32), kind: Kind) -> usize {
        if let Some(area) = self.area_of(world, start, kind) {
            return area;
        }
        let blocked = self.blocked.get_or_insert_with(|| world.blocked_cells());
        let area = world.reachable_cells(start, kind, blocked);
        self.areas.push((Reach::key(world, start, kind), area));
        self.areas.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `attack_goal` as it was before the reachable area: every candidate
    /// searched in turn.
    fn attack_goal_by_search(world: &World, entity: &Entity, target: &Entity) -> Option<Pos> {
        let range = world.weapon_range(entity);
        let radius = (range / FP).clamp(1, NAV_MAX_DESTINATION_RADIUS * 2);
        let target_cell = target.pos.cell_xy();
        let mut candidates = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let cell = (target_cell.0 + dx, target_cell.1 + dy);
                if !world.in_bounds(cell) || !world.cell_walkable_for(entity.kind, cell) {
                    continue;
                }
                let candidate = Pos::cell(cell.0, cell.1);
                if candidate.distance_sq(target.pos) <= i64::from(range).pow(2) {
                    candidates.push((
                        candidate.distance_sq(entity.pos),
                        candidate.distance_sq(target.pos),
                        cell.1,
                        cell.0,
                        candidate,
                    ));
                }
            }
        }
        candidates.sort_by_key(|c| (c.0, c.1, c.2, c.3));
        candidates.into_iter().find_map(|(_, _, _, _, candidate)| {
            (world.line_of_sight(candidate, target.pos, Some(entity.id))
                && !world
                    .find_path(entity.pos.cell_xy(), candidate.cell_xy(), entity.kind)
                    .is_empty())
            .then_some(candidate)
        })
    }

    #[test]
    fn an_army_sent_at_a_target_it_cannot_reach_does_not_stall_the_match() {
        // Trial 10: 27 Riveters ordered onto the Compact's headquarters
        // across deep water made every tick take seconds.
        let mut world = World::with_map(
            310,
            MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("three seats");
        world.ai_enabled = false;
        world.gate.tide = crate::Tide::Flood;
        let far = world
            .entities
            .iter()
            .find(|e| e.owner == 2 && e.kind == Kind::Headquarters)
            .expect("the third seat's headquarters")
            .id;
        let near = world
            .entities
            .iter()
            .find(|e| e.owner == 0 && e.kind == Kind::Headquarters)
            .expect("own headquarters")
            .clone();
        let (hx, hy) = near.pos.cell_xy();
        let mut army = Vec::new();
        for i in 0..20 {
            let cell = (hx + 6 + i % 5, hy + 6 + i / 5);
            if world.cell_walkable_for(Kind::Riveter, cell) {
                army.push(world.spawn_unit(0, Kind::Riveter, Pos::cell(cell.0, cell.1)));
            }
        }
        assert!(
            army.len() >= 15,
            "room for the army beside the headquarters"
        );
        // Across the flooded arms nothing is in reach; the old search and
        // the new one agree on that, and on a target on this side.
        let first = world.entity(army[0]).unwrap().clone();
        let target = world.entity(far).unwrap().clone();
        let range = world.weapon_range(&first);
        assert_eq!(
            world.attack_goal(&first, &target, range, 0, &mut Reach::default()),
            attack_goal_by_search(&world, &first, &target)
        );
        assert_eq!(attack_goal_by_search(&world, &first, &target), None);
        let enemy = world.spawn_unit(1, Kind::Reedguard, Pos::cell(hx + 14, hy + 2));
        let reachable = world.entity(enemy).unwrap().clone();
        let mut reach = Reach::default();
        let _ = world.attack_goal(&first, &target, range, 0, &mut reach);
        let found = world.attack_goal(&first, &reachable, range, 0, &mut reach);
        assert!(found.is_some(), "a target on this side is reached");
        assert_eq!(found, attack_goal_by_search(&world, &first, &reachable));
        world.entities.retain(|e| e.id != enemy);

        for id in &army {
            world.entity_mut(*id).unwrap().order = Order::Attack { target: far };
        }
        let started = std::time::Instant::now();
        for _ in 0..10 {
            world.step();
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(10),
            "ten ticks took {elapsed:?}"
        );
    }

    #[test]
    fn movement_rate_keeps_fractional_tick_steps() {
        let mut world = World::new(300, Faction::Union);
        let mut entity = world
            .entities
            .iter()
            .find(|e| e.kind == Kind::Hook)
            .expect("initial worker")
            .clone();
        entity.pos = Pos::cell(20, 20);
        let mut distance = 0;
        for tick in 0..TICK_HZ {
            world.tick = tick;
            distance += world.speed_per_tick(&entity);
        }
        assert_eq!(distance, spec(Kind::Hook).speed);
    }

    #[test]
    fn path_steps_to_neighbours_and_avoids_building_footprints() {
        let mut world = World::new(301, Faction::Union);
        world.ai_enabled = false;
        let building = world.spawn_building(0, Kind::Tower, Pos::cell(40, 40), 0);
        let path = world.find_path((35, 41), (47, 41), Kind::Hook);
        assert!(!path.is_empty());
        let mut previous = (35, 41);
        for cell in path {
            let (dx, dy) = ((cell.0 - previous.0).abs(), (cell.1 - previous.1).abs());
            assert_eq!(dx.max(dy), 1, "{previous:?} -> {cell:?}");
            assert!(!(40..42).contains(&cell.0) || !(40..42).contains(&cell.1));
            // A corner step never cuts the tower's corner.
            if dx == 1 && dy == 1 {
                for side in [(cell.0, previous.1), (previous.0, cell.1)] {
                    assert!(!(40..42).contains(&side.0) || !(40..42).contains(&side.1));
                }
            }
            previous = cell;
        }
        assert!(world.entities.iter().any(|entity| entity.id == building));
    }

    #[test]
    fn open_ground_is_crossed_on_the_diagonal() {
        let mut world = World::new(301, Faction::Union);
        world.ai_enabled = false;
        let start = (30, 30);
        let goal = (38, 38);
        assert!(world.cell_walkable_for(Kind::Hook, start));
        let clear = (0..=8).all(|offset| {
            (0..=8).all(|other| world.cell_walkable_for(Kind::Hook, (30 + offset, 30 + other)))
        });
        assert!(clear, "the test ground at seed 301 is open");
        let path = world.find_path(start, goal, Kind::Hook);
        assert_eq!(path.len(), 8, "eight corner steps, not sixteen side steps");
        assert!(
            path.iter()
                .enumerate()
                .all(|(index, cell)| { *cell == (31 + index as i32, 31 + index as i32) })
        );
    }

    #[test]
    fn a_machine_walks_around_one_standing_in_its_way() {
        // The one-cell side step turns by the machine's id; for this one it
        // pointed straight back the way it came, so before rules 16 the
        // machine stepped back, walked up to the standing one again and
        // repeated that for the rest of the match.
        let mut world = World::new(301, Faction::Union);
        world.ai_enabled = false;
        let open =
            (26..=40).all(|x| (24..=36).all(|y| world.cell_walkable_for(Kind::Hook, (x, y))));
        assert!(open, "the test ground at seed 301 is open");
        world.spawn_unit(0, Kind::Hook, Pos::cell(33, 30));
        let mut mover = world.spawn_unit(0, Kind::Hook, Pos::cell(36, 30));
        let mut spare = 0;
        while !mover.is_multiple_of(4) {
            world.spawn_unit(0, Kind::Hook, Pos::cell(20 + spare, 60));
            spare += 1;
            mover = world.spawn_unit(0, Kind::Hook, Pos::cell(36, 30));
        }
        let target = Pos::cell(29, 30);
        world
            .issue(
                0,
                Command::Move {
                    units: vec![mover],
                    target,
                    queued: false,
                },
            )
            .expect("move accepted");
        for _ in 0..(20 * TICK_HZ) {
            world.step();
            if world
                .entity(mover)
                .is_some_and(|entity| entity.pos == target)
            {
                return;
            }
        }
        let stuck = world.entity(mover).map(|entity| entity.pos.cell_xy());
        panic!("the mover never got past the standing machine; it is at {stuck:?}");
    }

    #[test]
    fn a_corner_step_is_as_long_as_a_side_step() {
        let speed = FP / 8;
        let (side, _) = move_towards(Pos::raw(0, 0), Pos::raw(FP, 0), speed);
        let (corner, _) = move_towards(Pos::raw(0, 0), Pos::raw(FP, FP), speed);
        assert_eq!(side.x, speed);
        let travelled = f64::from(corner.x).hypot(f64::from(corner.y));
        assert!((travelled - f64::from(speed)).abs() <= 1.5, "{corner:?}");
        // Eight corner cells take about 1.41 times as long as eight side cells.
        let ticks = |target: Pos| {
            let mut pos = Pos::raw(0, 0);
            let mut ticks = 0;
            loop {
                let (next, arrived) = move_towards(pos, target, speed);
                pos = next;
                ticks += 1;
                if arrived {
                    return ticks;
                }
            }
        };
        let side_ticks = ticks(Pos::raw(FP * 8, 0));
        let corner_ticks = ticks(Pos::raw(FP * 8, FP * 8));
        assert!(
            (140..=143).contains(&(corner_ticks * 100 / side_ticks)),
            "{corner_ticks}/{side_ticks}"
        );
    }

    #[test]
    fn placement_checks_entire_building_footprint() {
        let mut world = World::new(302, Faction::Union);
        world.ai_enabled = false;
        let existing = world.spawn_building(0, Kind::Tower, Pos::cell(40, 40), 0);
        assert!(!world.can_place(Kind::Works, Pos::cell(39, 40)));
        assert!(world.can_place(Kind::Works, Pos::cell(42, 42)));
        assert!(!world.can_place(Kind::Works, Pos::cell(41, 41)));
        assert!(world.entities.iter().any(|entity| entity.id == existing));
    }

    #[test]
    fn move_towards_never_snaps_a_half_cell() {
        let current = Pos::raw(0, 0);
        let target = Pos::raw(FP * 4, 0);
        let (next, arrived) = move_towards(current, target, FP / 2);
        assert_eq!(next.x, FP / 2);
        assert!(!arrived);
        let (last, arrived) = move_towards(Pos::raw(FP * 3 + FP / 2, 0), target, FP / 2);
        assert_eq!(last, target);
        assert!(arrived);
    }

    #[test]
    fn line_of_sight_respects_building_footprint_but_allows_endpoint() {
        let mut world = World::new(303, Faction::Union);
        world.ai_enabled = false;
        world.spawn_building(0, Kind::Works, Pos::cell(40, 40), 0);
        assert!(!world.line_of_sight(Pos::cell(35, 41), Pos::cell(47, 41), None));
        assert!(world.line_of_sight(Pos::cell(35, 41), Pos::cell(40, 41), None));
    }

    #[test]
    fn carried_workers_keep_distinct_dropoff_slots() {
        let mut world = World::new(304, Faction::Union);
        world.ai_enabled = false;
        let workers: Vec<u32> = world
            .entities
            .iter()
            .filter(|entity| entity.owner == 0 && entity.kind == Kind::Hook)
            .map(|entity| entity.id)
            .collect();
        let hq = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
            .cloned()
            .expect("starting HQ");
        for worker in &workers {
            let entity = world.entity_mut(*worker).expect("starting worker");
            entity.carried = 5;
            entity.order = Order::Gather { resource: 1 };
            entity.path_target = None;
        }
        let targets: Vec<Pos> = workers
            .iter()
            .map(|worker| {
                let pos = world.entity(*worker).expect("starting worker").pos;
                world.nearest_dropoff(0, pos).expect("HQ slot")
            })
            .collect();
        let unique = targets
            .iter()
            .map(|target| target.cell_xy())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), targets.len());
        for target in targets {
            assert!(world.is_interaction_slot(&hq, target));
        }
    }

    /// Trial 11-a (rules 20): six Compact machines stood beside the
    /// uncontested station with CAPTURE for three minutes and the capture
    /// never began.  Every capture order on the map was one formation, led
    /// by a Glazier far away on the south bank, so the others were given
    /// spots two to three cells out, outside the ring where capturing
    /// counts.  Returns the world after one CAPTURE to a Glazier far south
    /// of the station (spawned first, so it has the lowest id and led the
    /// old formation) and to machines on the given cells beside it.
    fn capture_from_one_command(offsets: &[(i32, i32)]) -> (World, Vec<u32>, u32) {
        let mut world = World::new(1101, Faction::Compact);
        world.ai_enabled = false;
        let (gx, gy) = world.map.gate_pos.cell_xy();
        let reachable = |world: &World, cell: (i32, i32)| {
            world.in_bounds(cell)
                && world.cell_walkable_for(Kind::Heliostat, cell)
                && !world.occupied_by_other(u32::MAX, cell)
                && !world.find_path(cell, (gx, gy), Kind::Heliostat).is_empty()
        };
        let far_cell = (14..40)
            .flat_map(|dy| (-6..=6).map(move |dx| (gx + dx, gy + dy)))
            .find(|cell| reachable(&world, *cell))
            .expect("a walkable cell far south of the station");
        let far = world.spawn_for_tests(0, Kind::Glazier, Pos::cell(far_cell.0, far_cell.1));
        let mut near = Vec::new();
        for (index, (dx, dy)) in offsets.iter().enumerate() {
            let cell = (gx + dx, gy + dy);
            assert!(
                reachable(&world, cell),
                "room beside the station at {cell:?}"
            );
            let kind = if index % 2 == 0 {
                Kind::Heliostat
            } else {
                Kind::Brander
            };
            near.push(world.spawn_for_tests(0, kind, Pos::cell(cell.0, cell.1)));
        }
        let mut units = vec![far];
        units.extend(near.iter().copied());
        world
            .issue(0, Command::Capture { units })
            .expect("capture order");
        (world, near, far)
    }

    /// Steps until the capture starts; whether it did within `seconds`.
    fn capture_starts_within(world: &mut World, seconds: u64) -> bool {
        for _ in 0..seconds * TICK_HZ {
            world.step();
            if world.gate.capture_player == Some(0) {
                return true;
            }
        }
        false
    }

    /// Every machine beside the station stands inside the ring with its
    /// order, and the far one is still on its way.
    fn assert_all_in_ring(world: &World, near: &[u32], far: u32) {
        let gate = world.map.gate_pos;
        let ring = i64::from(FP * 2).pow(2);
        for id in near {
            let entity = world.entity(*id).expect("near capturer");
            assert!(
                matches!(entity.order, Order::Capture),
                "{id} keeps its order: {:?}",
                entity.order
            );
            assert!(
                entity.pos.distance_sq(gate) <= ring,
                "{id} stands inside the capture ring, at {:?} (station {gate:?})",
                entity.pos
            );
        }
        assert!(
            world
                .entity(far)
                .expect("far capturer")
                .pos
                .distance_sq(gate)
                > ring,
            "the far machine has not arrived yet"
        );
    }

    #[test]
    fn capturers_beside_the_station_start_the_capture_while_one_is_far_away() {
        // Five machines north of the station.  Under rules 20 all of them
        // were sent to formation spots outside the ring and the capture
        // never started.
        let (mut world, near, far) =
            capture_from_one_command(&[(0, -4), (-1, -4), (1, -4), (0, -5), (-1, -5)]);
        assert!(
            capture_starts_within(&mut world, 3),
            "the capture starts within three seconds"
        );
        for _ in 0..5 * TICK_HZ {
            world.step();
        }
        assert_all_in_ring(&world, &near, far);
        assert_eq!(world.gate.capture_player, Some(0));
        assert!(world.gate.capture_progress > 0, "the capture runs");
    }

    #[test]
    fn capturers_on_both_sides_of_the_station_all_stand_in_the_ring() {
        // Six machines north and south of the station (the lane is five
        // cells wide there), three to four cells out.
        let (mut world, near, far) =
            capture_from_one_command(&[(-2, -3), (0, -4), (2, -3), (-2, 3), (0, 4), (2, 3)]);
        assert!(
            capture_starts_within(&mut world, 3),
            "the capture starts within three seconds"
        );
        for _ in 0..5 * TICK_HZ {
            world.step();
        }
        assert_all_in_ring(&world, &near, far);
    }
}
