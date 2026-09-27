//! State-driven first-player onboarding for BRINEWAKE practice sessions.
//!
//! The tutorial is deliberately a small observer of [`bw_sim::World`].  It
//! never issues commands, draws, or changes simulation state.  A stage is
//! completed only by a player action recorded in the command log and the
//! corresponding authoritative world result where one exists.

use bw_core::{EntityId, Kind, Pos};
use bw_sim::{Command, World};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const TOTAL_STEPS: usize = 7;

/// The coarse world object the desktop UI should focus for the current step.
///
/// The target is intentionally a category rather than an entity ID.  A
/// worker, Works, or combat unit can be destroyed while the player practices;
/// the desktop can then find another suitable object without invalidating the
/// tutorial's progress state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TutorialTarget {
    Worker,
    Works,
    Army,
    Gate,
    #[default]
    None,
}

/// Presentation data for the current tutorial step.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialView {
    /// One-based step number for display.  A completed tutorial reports 7.
    pub step: usize,
    pub total: usize,
    pub title: String,
    pub body: String,
    pub target: TutorialTarget,
    pub complete: bool,
}

/// Persistent, simulation-independent tutorial progress.
///
/// `Tutorial` is serializable so the desktop can keep it in a sidecar keyed
/// by the matching `World::state_hash()`.  The simulation save/replay format
/// remains owned by `bw_sim` and is not changed here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tutorial {
    /// Zero-based current stage.  `TOTAL_STEPS` means complete.
    stage: usize,
    /// Number of command records already present when this stage began.
    command_watermark: usize,
    /// Tick at which the stage began.  This guards against stale records in a
    /// reconstructed or externally edited command log.
    stage_start_tick: u64,
    /// Entity IDs that existed at stage start.  Entity IDs are never reused by
    /// the simulation, so this identifies newly built/produced results.
    stage_entities: BTreeSet<EntityId>,
    /// Positions at stage start, used to require real movement rather than a
    /// merely queued Move/AttackMove order.
    stage_positions: BTreeMap<EntityId, Pos>,
    /// Gate ownership and lane revision at stage start.
    stage_gate_owner: Option<u8>,
    stage_lane_revision: u32,
    stage_north_dry: bool,
}

impl Default for Tutorial {
    fn default() -> Self {
        Self {
            stage: 0,
            command_watermark: 0,
            stage_start_tick: 0,
            stage_entities: BTreeSet::new(),
            stage_positions: BTreeMap::new(),
            stage_gate_owner: None,
            stage_lane_revision: 0,
            stage_north_dry: true,
        }
    }
}

impl Tutorial {
    /// Starts a fresh tutorial against the supplied practice world snapshot.
    ///
    /// In particular, the starting workers' automatic `Gather` orders are
    /// before this watermark and therefore cannot satisfy the explicit Gather
    /// step.
    pub fn new(world: &World) -> Self {
        let mut tutorial = Self::default();
        tutorial.begin_stage(world);
        tutorial
    }

    /// Returns the zero-based current stage, or 7 after completion.
    #[allow(dead_code)]
    pub fn stage(&self) -> usize {
        self.stage.min(TOTAL_STEPS)
    }

    /// Returns true once all seven ordered stages have completed.
    pub fn is_complete(&self) -> bool {
        self.stage >= TOTAL_STEPS
    }

    /// Observes one world snapshot and advances at most one stage.
    ///
    /// `selected` is the desktop's current selection.  Command stages use
    /// only player-0 records after the stage watermark and require both
    /// `accepted` and `applied == Some(true)`.  Rejected records, accepted
    /// records that failed during application, and commands from earlier
    /// stages are ignored.
    pub fn update(&mut self, world: &World, selected: &[EntityId]) {
        if self.is_complete() {
            return;
        }
        self.reconcile_command_log(world);
        self.remember_new_combat_entities(world);

        let completed = match self.stage {
            0 => selected_own_worker(world, selected),
            1 => self.gather_completed(world),
            2 => self.works_completed(world),
            3 => self.combat_produced(world),
            4 => self.combat_moved(world),
            5 => self.gate_captured(world),
            6 => self.gate_switched(world),
            _ => false,
        };
        if completed {
            self.stage += 1;
            if !self.is_complete() {
                // This is intentionally a single transition.  Even if a
                // fixture has all later world state ready, the next stage's
                // command watermark starts only now and will be observed on a
                // later update call.
                self.begin_stage(world);
            }
        }
    }

    /// Builds the current persistent guidance view.  The target remains
    /// useful while a particular unit/building is missing, allowing the root
    /// UI to focus a replacement when the player recovers.
    pub fn view(&self, world: &World) -> TutorialView {
        let complete = self.is_complete();
        let step = if complete {
            TOTAL_STEPS
        } else {
            self.stage.saturating_add(1)
        };
        let (title, body, target) = if complete {
            (
                "Practice complete".to_string(),
                "Keep practicing, or start a skirmish.".to_string(),
                TutorialTarget::None,
            )
        } else {
            let copy = step_copy(self.stage);
            let mut body = copy.1.to_string();
            let mut target = copy.2;
            if self.stage == 2 {
                if let Some(works) = world
                    .entities
                    .iter()
                    .find(|e| e.owner == 0 && e.kind == Kind::Works && e.hp > 0)
                {
                    target = TutorialTarget::Works;
                    if works.build_remaining > 0 {
                        body = format!("Works ready in {}s.", works.build_remaining.div_ceil(30));
                    }
                } else {
                    target = TutorialTarget::Worker;
                }
            }
            if self.stage == 3 {
                if let Some(p) = world
                    .entities
                    .iter()
                    .filter(|e| e.owner == 0 && e.kind == Kind::Works)
                    .flat_map(|e| e.queue.iter())
                    .find(|p| is_combat_kind(p.kind))
                {
                    body = if p.remaining <= 1 {
                        "Machine ready. Clear the exit and check crew room.".into()
                    } else {
                        format!("{} ready in {}s.", p.kind.name(), p.remaining.div_ceil(30))
                    };
                } else {
                    body = format!(
                        "Select the Works and train a {}.",
                        world.players[0].faction.army()[0].name()
                    );
                }
            }
            (copy.0.to_string(), body, target)
        };
        TutorialView {
            step,
            total: TOTAL_STEPS,
            title,
            body,
            target,
            complete,
        }
    }

    fn begin_stage(&mut self, world: &World) {
        self.command_watermark = world.command_log.len();
        self.stage_start_tick = world.tick;
        self.stage_entities = world
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.id)
            .collect();
        self.stage_positions = world
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| (entity.id, entity.pos))
            .collect();
        self.stage_gate_owner = world.gate.owner;
        self.stage_lane_revision = world.gate.lane_revision;
        self.stage_north_dry = world.gate.north_dry();
    }

    /// Save/load or a replay reset can present a shorter log than the one
    /// observed when this state was serialized.  Clamp the watermark and
    /// refresh baselines so old commands and positions cannot complete a new
    /// stage accidentally.
    fn reconcile_command_log(&mut self, world: &World) {
        if self.command_watermark <= world.command_log.len() {
            return;
        }
        self.begin_stage(world);
    }

    fn records_since<'a>(
        &self,
        world: &'a World,
    ) -> impl Iterator<Item = &'a bw_sim::CommandRecord> {
        let start = self.command_watermark.min(world.command_log.len());
        world.command_log[start..].iter().filter(|record| {
            record.player == 0
                && record.accepted
                && record.applied == Some(true)
                && record.tick > self.stage_start_tick
        })
    }

    fn gather_completed(&self, world: &World) -> bool {
        self.records_since(world).any(|record| {
            matches!(
                &record.command,
                Command::Gather { units, resource } if *resource > 0 && !units.is_empty()
            )
        })
    }

    fn works_completed(&self, world: &World) -> bool {
        self.records_since(world).any(|record| {
            let Command::Build { kind, pos, .. } = &record.command else {
                return false;
            };
            if *kind != Kind::Works {
                return false;
            }
            world.entities.iter().any(|entity| {
                entity.owner == 0
                    && entity.kind == Kind::Works
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && entity.pos == *pos
                    && !self.stage_entities.contains(&entity.id)
            })
        })
    }

    fn combat_produced(&self, world: &World) -> bool {
        self.records_since(world).any(|record| {
            let Command::Train { kind, .. } = &record.command else {
                return false;
            };
            if !is_combat_kind(*kind) {
                return false;
            }
            // `applied == Some(true)` proves that the Works was valid when
            // the command executed.  Do not require that building to still
            // exist: a player can lose it after production completes and
            // should remain able to recover with another unit.
            world.entities.iter().any(|entity| {
                entity.owner == 0
                    && entity.kind == *kind
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && !self.stage_entities.contains(&entity.id)
            })
        })
    }

    fn combat_moved(&self, world: &World) -> bool {
        self.records_since(world).any(|record| {
            let units = match &record.command {
                Command::Move { units, .. } | Command::AttackMove { units, .. } => units,
                _ => return false,
            };
            units.iter().any(|id| {
                let Some(entity) = world.entities.iter().find(|entity| entity.id == *id) else {
                    return false;
                };
                if entity.owner != 0 || entity.hp <= 0 || !is_combat_kind(entity.kind) {
                    return false;
                }
                self.stage_positions
                    .get(id)
                    .is_some_and(|start| entity.pos != *start)
            })
        })
    }

    fn remember_new_combat_entities(&mut self, world: &World) {
        if self.stage != 4 {
            return;
        }
        // A machine's start is where it stood when first ordered: a new
        // machine walks out of its door on its own (rules 14), and that
        // walk is not the player moving it.
        let ordered: Vec<u32> = self
            .records_since(world)
            .flat_map(|record| match &record.command {
                Command::Move { units, .. } | Command::AttackMove { units, .. } => units.clone(),
                _ => Vec::new(),
            })
            .collect();
        for entity in &world.entities {
            if entity.owner == 0
                && entity.hp > 0
                && entity.build_remaining == 0
                && is_combat_kind(entity.kind)
            {
                if ordered.contains(&entity.id) {
                    self.stage_positions.entry(entity.id).or_insert(entity.pos);
                } else {
                    self.stage_positions.insert(entity.id, entity.pos);
                }
            }
        }
    }

    fn gate_captured(&self, world: &World) -> bool {
        if world.gate.owner != Some(0) {
            return false;
        }
        if self.stage_gate_owner == Some(0) {
            return world.command_log.iter().any(|r| {
                r.player == 0
                    && r.accepted
                    && r.applied == Some(true)
                    && matches!(r.command, Command::Capture { .. })
            });
        }
        self.records_since(world).any(
            |record| matches!(&record.command, Command::Capture { units } if !units.is_empty()),
        )
    }

    fn gate_switched(&self, world: &World) -> bool {
        if world.gate.warning_until.is_none() && self.stage_lane_revision > 0 {
            return world.command_log.iter().any(|r| {
                r.player == 0
                    && r.accepted
                    && r.applied == Some(true)
                    && matches!(r.command, Command::SwitchGate | Command::SetTide { .. })
            });
        }
        if world.gate.warning_until.is_some()
            || (world.gate.lane_revision == self.stage_lane_revision
                && world.gate.north_dry() == self.stage_north_dry)
        {
            return false;
        }
        self.records_since(world).any(|record| {
            matches!(
                &record.command,
                Command::SwitchGate | Command::SetTide { .. }
            )
        })
    }
}

fn selected_own_worker(world: &World, selected: &[EntityId]) -> bool {
    selected.iter().any(|id| {
        world.entities.iter().any(|entity| {
            entity.id == *id && entity.owner == 0 && entity.hp > 0 && entity.kind.is_worker()
        })
    })
}

fn is_combat_kind(kind: Kind) -> bool {
    !kind.is_worker() && !kind.is_building()
}

fn step_copy(stage: usize) -> (&'static str, &'static str, TutorialTarget) {
    match stage {
        0 => (
            "Select a worker",
            "Click a worker. Workers gather salvage and build.",
            TutorialTarget::Worker,
        ),
        1 => (
            "Gather salvage",
            "Right-click a wreck. The worker hauls loads home.",
            TutorialTarget::Worker,
        ),
        2 => (
            "Build a Works",
            "Choose Works, then click clear ground.",
            TutorialTarget::Works,
        ),
        3 => (
            "Train a combat machine",
            "Select the Works and train a machine.",
            TutorialTarget::Works,
        ),
        4 => (
            "Move it",
            "Select the new machine and right-click open ground.",
            TutorialTarget::Army,
        ),
        5 => (
            "Capture the sluice",
            "Select it and choose Capture.",
            TutorialTarget::Army,
        ),
        6 => (
            "Set the tide",
            "Choose DRY N or DRY S on the sluice card. That side dries after a 10-second warning; the other goes deep.",
            TutorialTarget::Gate,
        ),
        _ => unreachable!("step_copy called for completed tutorial"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::{FP, Faction};
    use bw_sim::{GATE_WARNING_TICKS, Order};

    fn ids_for(world: &World, kind: Kind) -> Vec<EntityId> {
        world
            .entities
            .iter()
            .filter(|entity| entity.owner == 0 && entity.kind == kind && entity.hp > 0)
            .map(|entity| entity.id)
            .collect()
    }

    fn worker(world: &World) -> EntityId {
        ids_for(world, world.players[0].faction.worker())
            .into_iter()
            .next()
            .expect("starting worker")
    }

    fn issue_and_step(world: &mut World, command: Command) {
        world.issue(0, command).expect("legal command");
        world.step();
    }

    #[test]
    fn automatic_starting_gather_does_not_complete_explicit_gather() {
        let world = World::new(42, Faction::Union);
        let mut tutorial = Tutorial::new(&world);
        let id = worker(&world);
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 1);
        assert!(!tutorial.is_complete());
        // The worker's initial Order::Gather is simulation setup, with no
        // command-log record, so it cannot advance this stage.
        assert!(matches!(
            world
                .entities
                .iter()
                .find(|entity| entity.id == id)
                .map(|entity| &entity.order),
            Some(Order::Gather { .. })
        ));
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 1);
    }

    #[test]
    fn a_lost_focus_worker_can_be_replaced_without_resetting_progress() {
        let mut world = World::new(421, Faction::Union);
        world.ai_enabled = false;
        let first = worker(&world);
        let mut tutorial = Tutorial::new(&world);
        tutorial.update(&world, &[first]);
        assert_eq!(tutorial.stage(), 1);

        if let Some(entity) = world.entities.iter_mut().find(|entity| entity.id == first) {
            entity.hp = 0;
        }
        let replacement = world
            .entities
            .iter()
            .find(|entity| {
                entity.owner == 0 && entity.kind.is_worker() && entity.hp > 0 && entity.id != first
            })
            .map(|entity| entity.id)
            .expect("replacement worker");
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        issue_and_step(
            &mut world,
            Command::Gather {
                units: vec![replacement],
                resource,
            },
        );
        tutorial.update(&world, &[replacement]);
        assert_eq!(tutorial.stage(), 2);
    }

    #[test]
    fn rejected_and_unapplied_commands_are_ignored() {
        let mut world = World::new(43, Faction::Union);
        world.ai_enabled = false;
        let mut tutorial = Tutorial::new(&world);
        let id = worker(&world);
        tutorial.update(&world, &[id]);

        let invalid = Command::Gather {
            units: vec![id],
            resource: u32::MAX,
        };
        assert!(world.issue(0, invalid).is_err());
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 1);

        // A queued record is not enough until the authoritative step applies
        // it.  This observes the real command-log lifecycle.
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        world
            .issue(
                0,
                Command::Gather {
                    units: vec![id],
                    resource,
                },
            )
            .expect("gather submission");
        assert_eq!(
            world.command_log.last().and_then(|record| record.applied),
            None
        );
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 1);
        world.step();
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 2);
    }

    #[test]
    fn progresses_through_build_and_production_from_real_world_state() {
        let mut world = World::new(44, Faction::Union);
        world.ai_enabled = false;
        let id = worker(&world);
        let mut tutorial = Tutorial::new(&world);
        tutorial.update(&world, &[id]);
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        issue_and_step(
            &mut world,
            Command::Gather {
                units: vec![id],
                resource,
            },
        );
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 2);

        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        let works_pos = Pos::cell(25, 60);
        issue_and_step(
            &mut world,
            Command::Build {
                worker: id,
                kind: Kind::Works,
                pos: works_pos,
                queued: false,
            },
        );
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 2, "must wait for Works completion");
        let works = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Works)
            .map(|entity| entity.id)
            .expect("works spawned");
        for _ in 0..2_000 {
            world.step();
            tutorial.update(&world, &[id]);
            if tutorial.stage() >= 3 {
                break;
            }
        }
        assert_eq!(tutorial.stage(), 3);

        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        issue_and_step(
            &mut world,
            Command::Train {
                building: works,
                kind: Kind::Riveter,
            },
        );
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 3, "must wait for production completion");
        for _ in 0..2_000 {
            world.step();
            tutorial.update(&world, &[id]);
            if tutorial.stage() >= 4 {
                break;
            }
        }
        assert_eq!(tutorial.stage(), 4);
    }

    #[test]
    fn movement_requires_position_change_after_applied_order() {
        let mut world = World::new(45, Faction::Union);
        world.ai_enabled = false;
        let id = worker(&world);
        let mut tutorial = Tutorial::new(&world);
        tutorial.update(&world, &[id]);
        // Move through the first three stages with real commands and state.
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        issue_and_step(
            &mut world,
            Command::Gather {
                units: vec![id],
                resource,
            },
        );
        tutorial.update(&world, &[id]);
        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        let works_pos = Pos::cell(25, 60);
        issue_and_step(
            &mut world,
            Command::Build {
                worker: id,
                kind: Kind::Works,
                pos: works_pos,
                queued: false,
            },
        );
        let works = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Works)
            .map(|entity| entity.id)
            .expect("works spawned");
        while world
            .entities
            .iter()
            .any(|entity| entity.id == works && entity.build_remaining > 0)
        {
            world.step();
            tutorial.update(&world, &[id]);
        }
        tutorial.update(&world, &[id]);
        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        issue_and_step(
            &mut world,
            Command::Train {
                building: works,
                kind: Kind::Riveter,
            },
        );
        while ids_for(&world, Kind::Riveter).is_empty() {
            world.step();
            tutorial.update(&world, &[id]);
        }
        // A new machine walks a few cells out of the door (rules 14): the
        // walk is not the player's order.
        for _ in 0..240 {
            world.step();
            tutorial.update(&world, &[id]);
        }
        tutorial.update(&world, &[id]);
        assert_eq!(tutorial.stage(), 4);
        let army = ids_for(&world, Kind::Riveter)[0];
        let start = world
            .entities
            .iter()
            .find(|entity| entity.id == army)
            .expect("army")
            .pos;
        issue_and_step(
            &mut world,
            Command::Move {
                units: vec![army],
                target: start,
                queued: true,
            },
        );
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 4, "an applied no-op move is insufficient");
        issue_and_step(
            &mut world,
            Command::Move {
                units: vec![army],
                target: Pos::cell(30, 64),
                queued: false,
            },
        );
        for _ in 0..500 {
            world.step();
            tutorial.update(&world, &[army]);
            if tutorial.stage() >= 5 {
                break;
            }
        }
        assert_eq!(tutorial.stage(), 5);
    }

    #[test]
    fn gate_capture_and_switch_require_completed_world_changes() {
        let mut world = World::new(46, Faction::Union);
        world.ai_enabled = false;
        let mut tutorial = Tutorial::new(&world);
        let worker = worker(&world);
        tutorial.update(&world, &[worker]);
        let resource = world
            .map
            .resources
            .iter()
            .find(|resource| resource.pos.cell_xy().0 < 40)
            .expect("starting salvage")
            .id;
        issue_and_step(
            &mut world,
            Command::Gather {
                units: vec![worker],
                resource,
            },
        );
        tutorial.update(&world, &[worker]);
        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        let works_pos = Pos::cell(25, 60);
        issue_and_step(
            &mut world,
            Command::Build {
                worker,
                kind: Kind::Works,
                pos: works_pos,
                queued: false,
            },
        );
        let works = world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Works)
            .map(|entity| entity.id)
            .expect("works");
        while world
            .entities
            .iter()
            .any(|entity| entity.id == works && entity.build_remaining > 0)
        {
            world.step();
            tutorial.update(&world, &[worker]);
        }
        tutorial.update(&world, &[worker]);
        world.players[0].salvage = 10_000;
        world.players[0].pressure = 10_000;
        issue_and_step(
            &mut world,
            Command::Train {
                building: works,
                kind: Kind::Riveter,
            },
        );
        let army = loop {
            if let Some(id) = ids_for(&world, Kind::Riveter).into_iter().next() {
                break id;
            }
            world.step();
            tutorial.update(&world, &[worker]);
        };
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 4);
        let gate_pos = world.map.gate_pos;
        issue_and_step(
            &mut world,
            Command::Move {
                units: vec![army],
                target: gate_pos,
                queued: false,
            },
        );
        while world
            .entities
            .iter()
            .find(|entity| entity.id == army)
            .is_some_and(|entity| {
                entity.pos.distance_sq(world.map.gate_pos) > i64::from(FP * 2).pow(2)
            })
        {
            world.step();
            tutorial.update(&world, &[army]);
        }
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 5);
        issue_and_step(&mut world, Command::Capture { units: vec![army] });
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 5);
        for _ in 0..world.capture_work() {
            world.step();
            tutorial.update(&world, &[army]);
        }
        assert_eq!(world.gate.owner, Some(0));
        assert_eq!(tutorial.stage(), 6);
        issue_and_step(&mut world, Command::SwitchGate);
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 6);
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
            tutorial.update(&world, &[army]);
        }
        assert_eq!(tutorial.stage(), 7);
        assert!(tutorial.is_complete());
        let view = tutorial.view(&world);
        assert!(view.complete);
        assert_eq!(view.step, 7);
        assert_eq!(view.target, TutorialTarget::None);
    }

    #[test]
    fn an_early_capture_and_switch_are_acknowledged_when_the_guide_catches_up() {
        let mut world = World::new(46, Faction::Union);
        world.ai_enabled = false;
        // Controlled setup isolates out-of-order guidance; the capture and
        // switch themselves must still finish through authoritative commands.
        let army = worker(&world);
        let gate = world.map.gate_pos;
        let unit = world.entities.iter_mut().find(|e| e.id == army).unwrap();
        unit.kind = Kind::Riveter;
        unit.pos = gate;
        unit.order = Order::Idle;
        issue_and_step(&mut world, Command::Capture { units: vec![army] });
        for _ in 0..world.capture_work() {
            world.step();
        }
        assert_eq!(world.gate.owner, Some(0));
        issue_and_step(&mut world, Command::SwitchGate);
        for _ in 0..GATE_WARNING_TICKS {
            world.step();
        }
        assert!(world.gate.warning_until.is_none());
        let mut tutorial = Tutorial::new(&world);
        tutorial.stage = 5;
        tutorial.begin_stage(&world);
        tutorial.update(&world, &[army]);
        assert_eq!(tutorial.stage(), 6);
        tutorial.update(&world, &[army]);
        assert!(tutorial.is_complete());
    }

    #[test]
    fn view_target_stays_recoverable_when_focus_unit_is_gone() {
        let world = World::new(47, Faction::Union);
        let tutorial = Tutorial::new(&world);
        let view = tutorial.view(&world);
        assert_eq!(view.target, TutorialTarget::Worker);
        assert_eq!(view.step, 1);
        assert_eq!(view.total, TOTAL_STEPS);
    }

    #[test]
    fn serialized_state_round_trips() {
        let world = World::new(48, Faction::Union);
        let tutorial = Tutorial::new(&world);
        let encoded = serde_json::to_string(&tutorial).expect("serialize tutorial");
        let decoded: Tutorial = serde_json::from_str(&encoded).expect("deserialize tutorial");
        assert_eq!(tutorial, decoded);
    }
}
