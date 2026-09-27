//! Presentation-side production selection and batch queueing.
//!
//! The simulation still owns the queue, resources and crew rules.  This
//! module only chooses among the producer entities the player selected and
//! submits ordinary `Command::Train` records.  In particular, it keeps a
//! small reservation view of commands that `World::issue` has accepted but
//! has not applied yet.  Without that view, a Shift-click batch would submit
//! several commands against the same pre-tick resources and queue length.
use crate::game::{Game, Mode};
use bw_content::{DOCTRINE_PRESSURE, DOCTRINE_SALVAGE, SURGE_PRESSURE, spec};
use bw_core::{EntityId, Faction, Kind};
use bw_sim::{Command, MAX_QUEUE};
use std::collections::BTreeMap;

const MAX_TRAIN_BATCH: usize = 5;

/// Why crew is short, in words that say what would raise it: at the
/// ceiling nothing does, below it the buildings that add crew.
pub(crate) fn crew_ceiling_reason(cap: u32) -> String {
    if cap >= bw_content::CREW_CAP_MAX {
        format!(
            "Crew is at the {} ceiling: no building raises it further.",
            bw_content::CREW_CAP_MAX
        )
    } else {
        format!(
            "Crew capacity is full: a Works adds {}, a Drydock {}, a Yard {}.",
            bw_content::CREW_CAP_WORKS,
            bw_content::CREW_CAP_DRYDOCK,
            bw_content::CREW_CAP_YARD
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct QueueReservation {
    slots: usize,
    work_ticks: u64,
    crew: u32,
}

#[derive(Clone, Debug, Default)]
struct PendingReservations {
    by_building: BTreeMap<EntityId, QueueReservation>,
    salvage: u32,
    pressure: u32,
}

#[derive(Clone, Copy, Debug)]
struct ProducerCandidate {
    id: EntityId,
    queue_slots: usize,
    queued_work_ticks: u64,
    queued_crew: u32,
}

impl Game {
    /// Return all completed, living, player-owned Works in deterministic ID
    /// order.  This is intentionally broader than a currently usable
    /// producer: a full queue is still a useful selection target for the F4
    /// recovery shortcut and for inspecting its status.
    pub(crate) fn works_ids(&self) -> Vec<EntityId> {
        let mut ids: Vec<_> = self
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.owner == 0
                    && entity.hp > 0
                    && entity.kind == Kind::Works
                    && entity.build_remaining == 0
            })
            .map(|entity| entity.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Select all completed Works without changing the camera.  This mirrors
    /// the other recovery helpers in `qol.rs` and clears stale gesture state
    /// so F4 is safe while an order mode or a chart drag is active.
    pub(crate) fn select_works(&mut self) {
        let ids = self.works_ids();
        if ids.is_empty() {
            self.notify("No completed Works. Select a worker and build one with B.");
            return;
        }
        self.selected = ids;
        self.mode = Mode::Context;
        self.drag = None;
        self.ux.minimap_drag = false;
        self.ux.keyboard_navigation = false;
        self.last_click = None;
        self.last_group = None;
        self.update_tutorial();
    }

    /// Resolve a production row hotkey from the current selection.
    ///
    /// Q has the familiar dual role: the first selected compatible producer
    /// chooses whether it means the worker row or the first combat row.  W
    /// and E address the second and third combat rows on the first selected
    /// Works.  The search deliberately ignores dead, foreign and unrelated
    /// buildings but keeps an unfinished producer eligible for the mapping so
    /// the resulting training message can explain why it is unavailable.
    pub(crate) fn production_hotkey_kind(&self, key: &str) -> Option<Kind> {
        let key = key.to_ascii_uppercase();
        let producer = self.selected.iter().find_map(|id| {
            let entity = self.world.entities.iter().find(|entity| entity.id == *id)?;
            (entity.owner == 0
                && entity.hp > 0
                && entity.aboard.is_none()
                && entity.kind.produces())
            .then_some(entity.kind)
        });
        let faction = self.world.players[0].faction;
        let row = match key.as_str() {
            "Q" => 0,
            "W" => 1,
            "E" => 2,
            "R" => 3,
            _ => return None,
        };
        match producer {
            Some(Kind::Headquarters) if row == 0 => Some(faction.worker()),
            Some(Kind::Headquarters) if row == 1 => Some(faction.scout()),
            Some(Kind::Works) => faction.army().get(row).copied(),
            Some(Kind::Drydock) => crate::trial11_controls::drydock_keys(faction)
                .get(row)
                .copied(),
            _ => None,
        }
    }

    /// Submit one or more training commands for the selected compatible
    /// producers.  `count` is capped at five because the batch affordance is
    /// a compact Shift-click action; callers can still request one unit for
    /// the ordinary click/hotkey behavior.
    pub(crate) fn train_selection(&mut self, kind: Kind, count: usize) {
        let requested = count.min(MAX_TRAIN_BATCH);
        if requested == 0 {
            return;
        }

        let mut queued = 0usize;
        let mut stop_reason = None;
        for _ in 0..requested {
            let reservations = pending_reservations(self);
            let Some(candidate) = self
                .producer_candidates(kind, &reservations)
                .into_iter()
                .find(|candidate| self.can_reserve_training(kind, candidate, &reservations))
            else {
                stop_reason = self.training_reason(kind);
                break;
            };

            // A two-player match sends the order to the network outbox, not
            // the world's log, so the acceptance comes from `issue` itself.
            if self.issue_accepted(Command::Train {
                building: candidate.id,
                kind,
            }) {
                queued += 1;
            } else {
                // Game::issue owns the normal dock acknowledgement and
                // immediate error notification.  Preserve its reason for a
                // truthful partial-batch summary.
                stop_reason = (!self.message.is_empty()).then(|| self.message.clone());
                break;
            }
        }

        if queued == 0 {
            self.notify(
                &stop_reason
                    .unwrap_or_else(|| format!("No eligible producer for {}.", kind.name())),
            );
        } else if requested == 1 {
            // `World::issue` already provides the legacy single-unit
            // acknowledgement, including its exact unit name.
        } else if queued < requested {
            let reason = stop_reason.unwrap_or_else(|| "No more producer capacity.".into());
            self.notify(&crate::trial11_controls::batch_words(
                kind.name(),
                queued,
                requested,
                &reason,
            ));
        } else {
            self.notify(&format!(
                "Queued {queued} {} across selected producers.",
                kind.name()
            ));
        }
    }

    /// Explain why one more unit of `kind` cannot currently be queued.  This
    /// is shared by disabled command buttons and by the batch preflight, so a
    /// partial batch reports the same state the player would see before the
    /// click.
    pub(crate) fn training_reason(&self, kind: Kind) -> Option<String> {
        let faction = self.world.players[0].faction;
        let selected: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| self.world.entities.iter().find(|entity| entity.id == *id))
            .filter(|entity| {
                entity.owner == 0
                    && entity.hp > 0
                    && entity.aboard.is_none()
                    && entity.kind.produces()
            })
            .collect();
        if selected.is_empty() {
            return Some("Select a completed headquarters, Works or Drydock.".into());
        }

        if !is_trainable_kind(faction, kind) {
            return Some(format!("{} is unavailable to this faction.", kind.name()));
        }

        let compatible: Vec<_> = selected
            .iter()
            .copied()
            .filter(|entity| producer_allows(entity.kind, faction, kind))
            .collect();
        if compatible.is_empty() {
            return Some(match bw_content::producer_of(faction, kind) {
                Some(Kind::Headquarters) if kind == faction.worker() => {
                    "Workers are trained at headquarters.".into()
                }
                Some(Kind::Headquarters) => {
                    format!("{} is trained at headquarters or a Drydock.", kind.name())
                }
                Some(Kind::Drydock) => format!("{} is trained at a Drydock.", kind.name()),
                _ => "Combat machines are trained at a Works.".into(),
            });
        }

        let reservations = pending_reservations(self);
        let completed: Vec<_> = compatible
            .iter()
            .copied()
            .filter(|entity| entity.build_remaining == 0)
            .collect();
        if completed.is_empty() {
            return Some("The selected production building is still under construction.".into());
        }

        let candidates = self.producer_candidates(kind, &reservations);
        if !candidates.is_empty()
            && candidates
                .iter()
                .all(|candidate| candidate.queue_slots >= MAX_QUEUE)
        {
            return Some("All selected production queues are full.".into());
        }

        if let Some(reason) = resource_training_reason(self, kind, &reservations) {
            return Some(reason);
        }

        let player = &self.world.players[0];
        let open_candidates: Vec<_> = candidates
            .iter()
            .filter(|candidate| candidate.queue_slots < MAX_QUEUE)
            .collect();
        if !open_candidates.is_empty()
            && open_candidates.iter().all(|candidate| {
                player
                    .crew
                    .saturating_add(candidate.queued_crew)
                    .saturating_add(spec(kind).crew)
                    > player.cap
            })
        {
            let required = open_candidates
                .iter()
                .map(|candidate| {
                    player
                        .crew
                        .saturating_add(candidate.queued_crew)
                        .saturating_add(spec(kind).crew)
                })
                .min()
                .unwrap_or_else(|| player.crew.saturating_add(spec(kind).crew));
            return Some(format!(
                "{} Needs {} free crew after queued production. Crew {}/{}.",
                crate::trial11_words::crew_ceiling_words(player.cap, self.faction),
                required.saturating_sub(player.cap),
                player.crew,
                player.cap
            ));
        }

        // If the producer list is empty for a reason other than queue
        // capacity, retain a useful generic explanation rather than enabling
        // a button that will immediately reject.
        if candidates.is_empty()
            || candidates
                .iter()
                .all(|candidate| !self.can_reserve_training(kind, candidate, &reservations))
        {
            Some("No selected producer can accept this unit yet.".into())
        } else {
            None
        }
    }

    fn producer_candidates(
        &self,
        kind: Kind,
        reservations: &PendingReservations,
    ) -> Vec<ProducerCandidate> {
        let faction = self.world.players[0].faction;
        let mut candidates: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| self.world.entities.iter().find(|entity| entity.id == *id))
            .filter(|entity| {
                entity.owner == 0
                    && entity.hp > 0
                    && entity.build_remaining == 0
                    && producer_allows(entity.kind, faction, kind)
            })
            .map(|entity| {
                let current_slots = entity.queue.len();
                let current_work_ticks = entity
                    .queue
                    .iter()
                    .map(|production| u64::from(production.remaining))
                    .sum::<u64>();
                let current_crew = entity
                    .queue
                    .iter()
                    .map(|production| spec(production.kind).crew)
                    .sum::<u32>();
                let pending = reservations
                    .by_building
                    .get(&entity.id)
                    .copied()
                    .unwrap_or_default();
                ProducerCandidate {
                    id: entity.id,
                    queue_slots: current_slots.saturating_add(pending.slots),
                    queued_work_ticks: current_work_ticks.saturating_add(pending.work_ticks),
                    queued_crew: current_crew.saturating_add(pending.crew),
                }
            })
            .collect();
        // Queue duration is the work players are trying to balance.  The ID
        // tie break keeps repeated batch submissions deterministic even when
        // multiple producers are equally empty.
        candidates.sort_by_key(|candidate| (candidate.queued_work_ticks, candidate.id));
        candidates
    }

    fn can_reserve_training(
        &self,
        kind: Kind,
        candidate: &ProducerCandidate,
        reservations: &PendingReservations,
    ) -> bool {
        if candidate.queue_slots >= MAX_QUEUE {
            return false;
        }
        let player = &self.world.players[0];
        let cost = spec(kind);
        let available_salvage = player.salvage.saturating_sub(reservations.salvage);
        let available_pressure = player.pressure.saturating_sub(reservations.pressure);
        if available_salvage < cost.salvage || available_pressure < cost.pressure {
            return false;
        }

        // Match the simulation's submission check: queued crew is counted on
        // the target producer, while the player's currently fielded crew is
        // shared.  Separate Works can therefore continue to queue work even
        // when another Works has a long queue.
        player
            .crew
            .saturating_add(candidate.queued_crew)
            .saturating_add(cost.crew)
            <= player.cap
    }
}

fn is_trainable_kind(faction: Faction, kind: Kind) -> bool {
    bw_content::producer_of(faction, kind).is_some()
}

fn producer_allows(producer: Kind, faction: Faction, unit: Kind) -> bool {
    bw_content::trains(faction, producer, unit)
}

/// The units ordered at `building` that the world has not queued yet: the
/// accepted orders still in the command log, and in a two-player match
/// those waiting in the session for their tick.
pub(crate) fn pending_trains(game: &Game, building: EntityId) -> Vec<Kind> {
    let logged = game
        .world
        .command_log
        .iter()
        .filter(|record| record.player == 0 && record.accepted && record.applied.is_none())
        .map(|record| record.command.clone());
    let sent = game
        .session
        .as_ref()
        .map(|session| session.pending_local())
        .unwrap_or_default();
    logged
        .chain(sent)
        .filter_map(|command| match command {
            Command::Train { building: at, kind } if at == building => Some(kind),
            _ => None,
        })
        .collect()
}

/// Salvage and pressure this seat's accepted orders will spend once the
/// world takes them up: the orders not yet applied.
pub(crate) fn pending_spend(game: &Game) -> (u32, u32) {
    let reservations = pending_reservations(game);
    (reservations.salvage, reservations.pressure)
}

fn pending_reservations(game: &Game) -> PendingReservations {
    let mut reservations = PendingReservations::default();
    let logged = game
        .world
        .command_log
        .iter()
        .filter(|record| record.player == 0 && record.accepted && record.applied.is_none())
        .map(|record| record.command.clone());
    // In a two-player match accepted orders wait in the session until their
    // tick; the world has not seen them.
    let sent = game
        .session
        .as_ref()
        .map(|session| session.pending_local())
        .unwrap_or_default();
    for command in logged.chain(sent) {
        match &command {
            Command::Train { building, kind } => {
                let cost = spec(*kind);
                let reservation = reservations.by_building.entry(*building).or_default();
                reservation.slots = reservation.slots.saturating_add(1);
                reservation.work_ticks = reservation
                    .work_ticks
                    .saturating_add(u64::from(cost.build_ticks.max(1)));
                reservation.crew = reservation.crew.saturating_add(cost.crew);
                reservations.salvage = reservations.salvage.saturating_add(cost.salvage);
                reservations.pressure = reservations.pressure.saturating_add(cost.pressure);
            }
            Command::Build { kind, .. } => {
                let cost = spec(*kind);
                reservations.salvage = reservations.salvage.saturating_add(cost.salvage);
                reservations.pressure = reservations.pressure.saturating_add(cost.pressure);
            }
            Command::Research { .. } => {
                reservations.salvage = reservations.salvage.saturating_add(DOCTRINE_SALVAGE);
                reservations.pressure = reservations.pressure.saturating_add(DOCTRINE_PRESSURE);
            }
            // An upgrade ordered in the same tick as another spends at its
            // tick too (trial 12: REFIT, TEMPER and OVERHAUL II in one tick,
            // the third refused two ticks after "started").
            Command::Upgrade { upgrade, .. } => {
                let (salvage, pressure, _) = upgrade.cost();
                reservations.salvage = reservations.salvage.saturating_add(salvage);
                reservations.pressure = reservations.pressure.saturating_add(pressure);
            }
            Command::Surge { units } => {
                reservations.pressure = reservations
                    .pressure
                    .saturating_add(SURGE_PRESSURE.saturating_mul(units.len() as u32));
            }
            _ => {}
        }
    }
    reservations
}

fn resource_training_reason(
    game: &Game,
    kind: Kind,
    reservations: &PendingReservations,
) -> Option<String> {
    let player = &game.world.players[0];
    let cost = spec(kind);
    let available_salvage = player.salvage.saturating_sub(reservations.salvage);
    let available_pressure = player.pressure.saturating_sub(reservations.pressure);
    let salvage = cost.salvage.saturating_sub(available_salvage);
    let pressure = cost.pressure.saturating_sub(available_pressure);
    // The figures explain the refusal: the header showed 137 while the
    // queue had already spoken for 140.
    match (salvage, pressure) {
        (0, 0) => None,
        (s, 0) => Some(format!(
            "Need {s} more salvage: {} on hand, {} pending.",
            player.salvage, reservations.salvage
        )),
        (0, p) => Some(format!(
            "Need {p} more pressure: {} on hand, {} pending.",
            player.pressure, reservations.pressure
        )),
        (s, p) => Some(format!(
            "Need {s} more salvage and {p} pressure after pending orders."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Pos;
    use bw_sim::{Order, Production, World};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fresh_game(label: &str, faction: Faction) -> (Game, PathBuf) {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let data = std::env::temp_dir().join(format!(
            "brinewake-production-qol-{label}-{}-{stamp}",
            std::process::id()
        ));
        let mut game = Game::new_with_data_dir(base, data.clone());
        game.faction = faction;
        game.start();
        game.world.ai_enabled = false;
        (game, data)
    }

    fn cleanup(path: PathBuf) {
        let _ = std::fs::remove_dir_all(path);
    }

    fn add_works(game: &mut Game, count: usize) -> Vec<EntityId> {
        let hq = game
            .world
            .entities
            .iter()
            .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
            .cloned()
            .expect("starting headquarters");
        let start_id = game
            .world
            .entities
            .iter()
            .map(|entity| entity.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut ids = Vec::new();
        for index in 0..count {
            let mut works = hq.clone();
            works.id = start_id.saturating_add(index as u32);
            works.kind = Kind::Works;
            works.pos = Pos::cell(25 + index as i32 * 5, 35);
            works.hp = spec(Kind::Works).health;
            works.max_hp = works.hp;
            works.build_remaining = 0;
            works.queue.clear();
            works.rally = None;
            works.builder = None;
            works.order = Order::Idle;
            works.path.clear();
            works.waypoints.clear();
            works.path_target = None;
            works.path_index = 0;
            works.path_lane_revision = game.world.gate.lane_revision;
            game.world.entities.push(works);
            ids.push(start_id.saturating_add(index as u32));
        }
        game.world.entities.sort_by_key(|entity| entity.id);
        game.world.reset_fixture_origin().expect("fixture origin");
        ids
    }

    fn pending_train_count(game: &Game) -> usize {
        game.world
            .command_log
            .iter()
            .filter(|record| {
                record.player == 0
                    && record.accepted
                    && record.applied.is_none()
                    && matches!(record.command, Command::Train { .. })
            })
            .count()
    }

    #[test]
    fn both_factions_expose_works_selection_and_row_hotkeys() {
        for faction in [Faction::Union, Faction::Assembly] {
            let (mut game, dir) = fresh_game("hotkeys", faction);
            let works = add_works(&mut game, 2);
            let hq = game
                .world
                .entities
                .iter()
                .find(|entity| entity.owner == 0 && entity.kind == Kind::Headquarters)
                .map(|entity| entity.id)
                .expect("headquarters");

            game.selected = vec![hq];
            assert_eq!(
                game.production_hotkey_kind("q"),
                Some(faction.worker()),
                "worker row {faction:?}"
            );
            assert_eq!(game.production_hotkey_kind("w"), Some(faction.scout()));

            game.selected = works.clone();
            assert_eq!(game.production_hotkey_kind("Q"), Some(faction.army()[0]));
            assert_eq!(game.production_hotkey_kind("W"), Some(faction.army()[1]));
            assert_eq!(game.production_hotkey_kind("E"), Some(faction.army()[2]));
            game.select_works();
            assert_eq!(game.selected, works);
            cleanup(dir);
        }
    }

    #[test]
    fn batch_balances_work_and_reports_partial_resource_budget() {
        for faction in [Faction::Union, Faction::Assembly] {
            let (mut game, dir) = fresh_game("distribution", faction);
            let works = add_works(&mut game, 2);
            let kind = faction.army()[0];
            let cost = spec(kind);
            game.selected = works.clone();
            game.world.players[0].salvage = cost.salvage.saturating_mul(2);
            game.world.players[0].pressure = cost.pressure.saturating_mul(2);
            game.world.players[0].cap = 80;
            game.train_selection(kind, 5);
            assert_eq!(pending_train_count(&game), 2);
            assert!(game.message.contains("2 of 5"), "{}", game.message);
            let queues: Vec<_> = works
                .iter()
                .map(|id| {
                    game.world
                        .command_log
                        .iter()
                        .filter(|record| {
                            record.player == 0
                                && record.accepted
                                && record.applied.is_none()
                                && matches!(
                                    record.command,
                                    Command::Train { building, .. } if building == *id
                                )
                        })
                        .count()
                })
                .collect();
            assert_eq!(queues, vec![1, 1], "equal work is distributed first");
            cleanup(dir);
        }
    }

    #[test]
    fn full_dead_foreign_and_incomplete_works_are_filtered_for_training() {
        let (mut game, dir) = fresh_game("filters", Faction::Union);
        let works = add_works(&mut game, 4);
        let kind = Faction::Union.army()[0];
        let cost = spec(kind);
        game.world.players[0].salvage = 10_000;
        game.world.players[0].pressure = 10_000;

        game.world
            .entities
            .iter_mut()
            .find(|entity| entity.id == works[0])
            .unwrap()
            .queue = (0..MAX_QUEUE)
            .map(|_| Production {
                kind,
                remaining: cost.build_ticks,
                started: false,
                cost_salvage: cost.salvage,
                cost_pressure: cost.pressure,
            })
            .collect();
        game.world
            .entities
            .iter_mut()
            .find(|entity| entity.id == works[1])
            .unwrap()
            .hp = 0;
        game.world
            .entities
            .iter_mut()
            .find(|entity| entity.id == works[2])
            .unwrap()
            .owner = 1;
        game.world
            .entities
            .iter_mut()
            .find(|entity| entity.id == works[3])
            .unwrap()
            .build_remaining = 10;
        game.selected = works.clone();
        assert_eq!(game.works_ids(), vec![works[0]]);
        game.train_selection(kind, 1);
        assert_eq!(pending_train_count(&game), 0);
        assert!(game.message.contains("No eligible") || game.message.contains("full"));
        cleanup(dir);
    }

    #[test]
    fn pending_burst_reserves_queue_resources_and_replays_exactly() {
        let (mut game, dir) = fresh_game("pending", Faction::Assembly);
        let works = add_works(&mut game, 1);
        let kind = Faction::Assembly.army()[2];
        let cost = spec(kind);
        game.selected = works.clone();
        game.world.players[0].salvage = cost.salvage.saturating_mul(5);
        game.world.players[0].pressure = cost.pressure.saturating_mul(5);
        game.world
            .reset_fixture_origin()
            .expect("budget fixture origin");
        for _ in 0..3 {
            game.world
                .issue(
                    0,
                    Command::Train {
                        building: works[0],
                        kind,
                    },
                )
                .expect("pending train");
        }
        game.train_selection(kind, 5);
        // Five total commands fit the reserved resources, although only two
        // were submitted by the planner after the pre-existing burst.
        assert_eq!(pending_train_count(&game), 5);
        assert!(game.message.contains("2 of 5"), "{}", game.message);
        game.tick();
        std::fs::create_dir_all(&dir).expect("replay directory");
        let replay_path = dir.join("production.replay.json");
        game.world
            .export_replay(&replay_path)
            .expect("write replay");
        let replayed = World::replay(&replay_path).expect("replay");
        assert_eq!(replayed.state_hash(), game.world.state_hash());
        cleanup(dir);
    }
}
