//! Presentation state for BRINEWAKE's dockmaster feedback and result log.
//!
//! The simulation remains authoritative.  A submitted command only becomes an
//! [`TelegraphStatus::Accepted`] entry after the matching `CommandRecord` has
//! `applied == Some(true)` at an authoritative tick.  The same path handles a
//! same-tick rejection.  Event collection below deliberately consumes only
//! public gate events, player-owned economy events, and deaths the player can
//! know about; it never reads hidden enemy production or combat state.

use crate::canvas::{
    Atlas, COBALT, Canvas, Color, EDGE, GOLD, INK, JADE, MUTED, PANEL, RED, WHITE,
};
use bw_content::Doctrine;
use bw_core::{Faction, Pos};
use bw_sim::{CommandRecord, Event, EventKind, Outcome, World};
use serde::{Deserialize, Serialize};

/// Maximum number of observed entries retained in a match's dock log.
pub const MAX_DOCK_LOG_ENTRIES: usize = 160;
/// Deaths are the most numerous events by far. They have their own, smaller
/// bound so they can never push the captures, switches and counts that
/// decided the match out of the log.
pub const MAX_DOCK_LOG_DEATHS: usize = 64;
/// Maximum number of submitted commands kept while they await an authoritative
/// apply/reject result.  The visual telegraph still shows the newest one.
pub const MAX_PENDING_COMMANDS: usize = 32;

/// The four visual states used by the dock telegraph artwork.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelegraphStatus {
    #[default]
    Idle,
    Pending,
    Accepted,
    Rejected,
}

impl TelegraphStatus {
    /// Exact atlas key expected by the root-authored telegraph strip.
    pub const fn asset_key(self) -> &'static str {
        match self {
            Self::Idle => "dock_telegraph_idle",
            Self::Pending => "dock_telegraph_pending",
            Self::Accepted => "dock_telegraph_accepted",
            Self::Rejected => "dock_telegraph_rejected",
        }
    }

    const fn compact_label(self) -> &'static str {
        match self {
            Self::Idle => "READY",
            Self::Pending => "WAIT",
            Self::Accepted => "ACK",
            Self::Rejected => "REJECT",
        }
    }
}

/// Small command acknowledgement shown near the existing command console.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockTelegraph {
    pub status: TelegraphStatus,
    /// Human-readable action name, for example `MOVE` or `BUILD WORKS`.
    pub caption: String,
    /// A user-facing rejection reason, when the status is `Rejected`.
    pub reason: Option<String>,
    /// Destination carried by the command, if it has one.  The world marker
    /// remains the authoritative destination stamp; this is only feedback
    /// metadata for callers that want to describe the command.
    pub target: Option<Pos>,
    pub submitted_tick: u64,
    pub acknowledged_tick: Option<u64>,
    pub sequence: Option<u64>,
    /// Monotonic UI interaction generation. It prevents an older pending
    /// command from replacing a newer immediate rejection submitted on the
    /// same simulation tick.
    #[serde(default)]
    pub generation: u64,
}

impl Default for DockTelegraph {
    fn default() -> Self {
        Self {
            status: TelegraphStatus::Idle,
            caption: "NO ORDER".into(),
            reason: None,
            target: None,
            submitted_tick: 0,
            acknowledged_tick: None,
            sequence: None,
            generation: 0,
        }
    }
}

impl DockTelegraph {
    pub fn asset_key(&self) -> &'static str {
        self.status.asset_key()
    }
}

/// Categories retained by the bounded observed log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockLogKind {
    GateWarning,
    GateSwitch,
    GateClaim,
    Construction,
    Research,
    Death,
    Outcome,
    /// A hold count started or ran out. The gauges are public.
    Hold,
    /// A seat of three was knocked out, with where it placed.
    Out,
}

impl DockLogKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::GateWarning => "SLUICE WARNING",
            Self::GateSwitch => "SLUICE SWITCH",
            Self::GateClaim => "SLUICE CLAIMED",
            Self::Construction => "CONSTRUCTION",
            Self::Research => "RESEARCH",
            Self::Death => "VISIBLE DEATH",
            Self::Outcome => "MATCH RESULT",
            Self::Hold => "HOLD COUNT",
            Self::Out => "SEAT OUT",
        }
    }

    pub const fn chart_color(self) -> Color {
        match self {
            Self::GateWarning | Self::GateSwitch | Self::GateClaim | Self::Hold => GOLD,
            Self::Construction | Self::Research => JADE,
            Self::Death => RED,
            Self::Outcome | Self::Out => WHITE,
        }
    }

    pub const fn is_chart_major(self) -> bool {
        matches!(
            self,
            Self::GateWarning
                | Self::GateSwitch
                | Self::GateClaim
                | Self::Construction
                | Self::Research
                | Self::Death
                | Self::Outcome
                | Self::Hold
                | Self::Out
        )
    }
}

/// One event the player was entitled to observe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockLogEntry {
    pub tick: u64,
    pub kind: DockLogKind,
    pub label: String,
    pub detail: String,
    pub pos: Option<Pos>,
}

impl Default for DockLogEntry {
    fn default() -> Self {
        Self {
            tick: 0,
            kind: DockLogKind::Outcome,
            label: DockLogKind::Outcome.label().into(),
            detail: String::new(),
            pos: None,
        }
    }
}

impl DockLogEntry {
    pub fn new(tick: u64, kind: DockLogKind, detail: impl Into<String>, pos: Option<Pos>) -> Self {
        Self {
            tick,
            kind,
            label: kind.label().into(),
            detail: detail.into(),
            pos,
        }
    }
}

/// Bounded, serializable history of public and observed match events.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DockLog {
    pub entries: Vec<DockLogEntry>,
    /// `false` distinguishes a missing or hash-mismatched UI sidecar from a
    /// new match that simply has not emitted an observed event yet.
    pub history_available: bool,
    /// A valid save may contain only events observed after its sidecar was
    /// created. Keep that provenance visible even as new events arrive.
    #[serde(default)]
    pub history_partial: bool,
    #[serde(default)]
    pub history_start_tick: Option<u64>,
    /// Set once old entries have been evicted by the 64-entry bound.
    #[serde(default)]
    pub recent_only: bool,
}

impl DockLog {
    pub fn mark_available(&mut self) {
        self.history_available = true;
    }

    /// Mark an empty log as a complete fresh-match history.
    pub fn mark_fresh(&mut self) {
        self.history_available = true;
        self.history_partial = false;
        self.history_start_tick = None;
        self.recent_only = false;
    }

    /// Mark history as unavailable before a matching sidecar is found. New
    /// observations remain useful but are labeled partial and tied to this
    /// load tick.
    pub fn mark_missing(&mut self, start_tick: u64) {
        self.entries.clear();
        self.history_available = false;
        self.history_partial = true;
        self.history_start_tick = Some(start_tick);
        self.recent_only = false;
    }

    pub fn push(&mut self, entry: DockLogEntry) {
        self.history_available = true;
        self.entries.push(entry);
        self.trim_to_limit();
    }

    pub fn trim_to_limit(&mut self) {
        let mut deaths = self
            .entries
            .iter()
            .filter(|entry| entry.kind == DockLogKind::Death)
            .count();
        while deaths > MAX_DOCK_LOG_DEATHS {
            let Some(oldest) = self
                .entries
                .iter()
                .position(|entry| entry.kind == DockLogKind::Death)
            else {
                break;
            };
            self.entries.remove(oldest);
            deaths -= 1;
            self.recent_only = true;
        }
        if self.entries.len() > MAX_DOCK_LOG_ENTRIES {
            let excess = self.entries.len() - MAX_DOCK_LOG_ENTRIES;
            self.entries.drain(..excess);
            self.recent_only = true;
        }
    }

    pub fn record(
        &mut self,
        tick: u64,
        kind: DockLogKind,
        detail: impl Into<String>,
        pos: Option<Pos>,
    ) {
        self.push(DockLogEntry::new(tick, kind, detail, pos));
    }

    /// Text used when a result screen cannot recover the sidecar history.
    pub fn history_label(&self) -> String {
        let count = |recent: bool| {
            format!(
                "{}{} EVENT{}",
                self.entries.len(),
                if recent { " RECENT" } else { "" },
                if self.entries.len() == 1 { "" } else { "S" }
            )
        };
        if self.history_partial {
            let since = self
                .history_start_tick
                .map(|tick| format!(" {}", format_tick(tick)))
                .unwrap_or_default();
            if self.entries.is_empty() {
                format!("LOG SINCE LOAD{}", since)
            } else {
                format!("LOG SINCE LOAD{} / {}", since, count(true))
            }
        } else if !self.history_available {
            "NO MATCH LOG".into()
        } else if self.entries.is_empty() {
            "NO EVENTS YET".into()
        } else {
            count(self.recent_only)
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct DockLogWire {
    entries: Vec<DockLogEntry>,
    history_available: bool,
    history_partial: bool,
    history_start_tick: Option<u64>,
    recent_only: bool,
}

impl<'de> Deserialize<'de> for DockLog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DockLogWire::deserialize(deserializer)?;
        let mut log = Self {
            entries: wire.entries,
            history_available: wire.history_available,
            history_partial: wire.history_partial,
            history_start_tick: wire.history_start_tick,
            recent_only: wire.recent_only,
        };
        log.trim_to_limit();
        Ok(log)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingCommand {
    player: u8,
    sequence: u64,
    caption: String,
    target: Option<Pos>,
    submitted_tick: u64,
    generation: u64,
}

/// All presentation-only dock state that is persisted alongside a matching
/// simulation save.  The pending queue and processed tick are intentionally
/// transient; command state is recovered from `World::command_log` and a
/// loaded save starts with no stale in-flight pointer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockState {
    pub telegraph: DockTelegraph,
    pub log: DockLog,
    /// The simulation's words for the result ("... HOLDS THE TIDE", "...
    /// WINS"), so the result screen names the cause.
    pub outcome_text: String,
    #[serde(skip)]
    pending: Vec<PendingCommand>,
    #[serde(skip)]
    processed_world_tick: Option<u64>,
    #[serde(skip)]
    next_generation: u64,
    /// Both hold gauges at the last tick, and each count's peak so far.
    #[serde(skip)]
    last_hold: Option<Vec<u32>>,
    #[serde(skip)]
    hold_peak: Vec<u32>,
}

impl DockState {
    /// Clear history, acknowledgement, and transient correlation state for a
    /// new match or practice field.
    pub fn reset(&mut self) {
        self.outcome_text.clear();
        *self = Self::default();
    }

    /// Start a fresh match with an explicitly complete empty history.
    pub fn reset_for_new_match(&mut self) {
        self.reset();
        self.log.mark_fresh();
    }

    /// Clear stale entries while retaining a provenance marker for a loaded
    /// world whose UI sidecar was absent or did not match its hash.
    pub fn mark_missing_history(&mut self, start_tick: u64) {
        self.pending.clear();
        self.processed_world_tick = None;
        self.next_generation = 0;
        self.telegraph = DockTelegraph::default();
        self.log.mark_missing(start_tick);
    }

    /// Record a successful submission as `Pending`.  This must not be called
    /// for an immediate `World::issue` error; use
    /// [`Self::on_command_rejected`] in that case.
    pub fn on_command_submitted(
        &mut self,
        tick: u64,
        sequence: u64,
        caption: impl Into<String>,
        target: Option<Pos>,
    ) {
        let caption = caption.into();
        self.next_generation = self.next_generation.wrapping_add(1);
        let generation = self.next_generation;
        self.log.mark_available();
        self.pending.push(PendingCommand {
            player: 0,
            sequence,
            caption: caption.clone(),
            target,
            submitted_tick: tick,
            generation,
        });
        if self.pending.len() > MAX_PENDING_COMMANDS {
            let excess = self.pending.len() - MAX_PENDING_COMMANDS;
            self.pending.drain(..excess);
        }
        self.telegraph = DockTelegraph {
            status: TelegraphStatus::Pending,
            caption,
            reason: None,
            target,
            submitted_tick: tick,
            acknowledged_tick: None,
            sequence: Some(sequence),
            generation,
        };
    }

    /// Record an immediate, authoritative rejection returned from
    /// `World::issue`.  A later rejection from `World::step` is handled by
    /// [`Self::after_authoritative_tick`].
    pub fn on_command_rejected(
        &mut self,
        tick: u64,
        sequence: Option<u64>,
        caption: impl Into<String>,
        reason: impl Into<String>,
        target: Option<Pos>,
    ) {
        self.next_generation = self.next_generation.wrapping_add(1);
        let generation = self.next_generation;
        self.log.mark_available();
        self.telegraph = DockTelegraph {
            status: TelegraphStatus::Rejected,
            caption: caption.into(),
            reason: Some(reason.into()),
            target,
            submitted_tick: tick,
            acknowledged_tick: Some(tick),
            sequence,
            generation,
        };
    }

    /// Rebuild the transient pending queue after loading a matching UI
    /// sidecar. A serialized `Pending` pointer is retained only when the
    /// authoritative world still has the same command with `applied == None`;
    /// an already-resolved record is reflected immediately instead.
    pub fn recover_from_world(&mut self, world: &World) {
        self.pending.clear();
        self.processed_world_tick = None;
        self.next_generation = self.telegraph.generation;
        if self.telegraph.status != TelegraphStatus::Pending {
            return;
        }
        let Some(sequence) = self.telegraph.sequence else {
            self.telegraph = DockTelegraph::default();
            return;
        };
        let Some(record) = world
            .command_log
            .iter()
            .find(|record| record.player == 0 && record.sequence == sequence)
        else {
            // The pointer has no authoritative counterpart in this save. Keep
            // the result honest instead of displaying a forever-pending order.
            self.telegraph = DockTelegraph {
                status: TelegraphStatus::Rejected,
                caption: self.telegraph.caption.clone(),
                reason: Some("no authoritative acknowledgement in this save".into()),
                target: self.telegraph.target,
                submitted_tick: self.telegraph.submitted_tick,
                acknowledged_tick: Some(world.tick),
                sequence: Some(sequence),
                generation: self.telegraph.generation,
            };
            return;
        };
        match record.applied {
            None if record.accepted => {
                self.pending.push(PendingCommand {
                    player: 0,
                    sequence,
                    caption: self.telegraph.caption.clone(),
                    target: self.telegraph.target,
                    submitted_tick: self.telegraph.submitted_tick,
                    generation: self.telegraph.generation,
                });
            }
            Some(applied) => {
                self.telegraph.status = if applied {
                    TelegraphStatus::Accepted
                } else {
                    TelegraphStatus::Rejected
                };
                self.telegraph.reason = if applied { None } else { record.reason.clone() };
                self.telegraph.acknowledged_tick = Some(record.tick);
            }
            None => {
                self.telegraph.status = TelegraphStatus::Rejected;
                self.telegraph.reason = Some("submission was not accepted in this save".into());
                self.telegraph.acknowledged_tick = Some(world.tick);
            }
        }
    }

    /// Consume the authoritative result of the most recent simulation tick.
    /// This method is intentionally the only path that changes a pending
    /// telegraph to `Accepted`.
    pub fn after_authoritative_tick(&mut self, world: &World) {
        if self.processed_world_tick == Some(world.tick) {
            return;
        }
        self.processed_world_tick = Some(world.tick);
        self.log.mark_available();

        let mut resolved = Vec::new();
        for (index, pending) in self.pending.iter().enumerate() {
            let Some(record) = find_record(world, pending) else {
                continue;
            };
            let Some(applied) = record.applied else {
                continue;
            };
            resolved.push((index, pending.clone(), record.clone(), applied));
        }

        for (index, _, _, _) in resolved.iter().rev() {
            self.pending.remove(*index);
        }

        let newest_pending = self.pending.iter().max_by_key(|pending| pending.generation);
        if let Some(pending) = newest_pending
            && pending.generation > self.telegraph.generation
        {
            self.telegraph = DockTelegraph {
                status: TelegraphStatus::Pending,
                caption: pending.caption.clone(),
                reason: None,
                target: pending.target,
                submitted_tick: pending.submitted_tick,
                acknowledged_tick: None,
                sequence: Some(pending.sequence),
                generation: pending.generation,
            };
        } else if let Some((_, pending, record, applied)) = resolved
            .into_iter()
            .max_by_key(|(_, pending, record, _)| (pending.generation, record.sequence))
            .filter(|(_, pending, _, _)| pending.generation >= self.telegraph.generation)
        {
            let status = if applied {
                TelegraphStatus::Accepted
            } else {
                TelegraphStatus::Rejected
            };
            self.telegraph = DockTelegraph {
                status,
                caption: pending.caption,
                reason: if applied { None } else { record.reason },
                target: pending.target,
                submitted_tick: pending.submitted_tick,
                acknowledged_tick: Some(record.tick),
                sequence: Some(record.sequence),
                generation: pending.generation,
            };
        }

        self.record_observed_events(world);
    }

    fn record_observed_events(&mut self, world: &World) {
        for event in &world.events {
            match event.kind {
                // Gate warnings, ownership changes, and lane changes are
                // declared public by the simulation contract.
                EventKind::GateWarning => self.log.record(
                    event.tick,
                    DockLogKind::GateWarning,
                    safe_detail(&event.text),
                    event.to.or(event.from),
                ),
                EventKind::GateChanged => self.log.record(
                    event.tick,
                    DockLogKind::GateSwitch,
                    safe_detail(&event.text),
                    event.to.or(event.from),
                ),
                EventKind::GateCaptured => self.log.record(
                    event.tick,
                    DockLogKind::GateClaim,
                    safe_detail(&claim_words(world, event)),
                    event.to.or(event.from),
                ),
                // Economy events carry no hidden enemy detail once ownership
                // is checked.  Research cancellation is included because it
                // changes the player's visible doctrine path.
                EventKind::BuildStarted | EventKind::BuildCompleted if event.player == Some(0) => {
                    self.log.record(
                        event.tick,
                        DockLogKind::Construction,
                        safe_detail(&crate::trial11_words::own_building_text(
                            &event.text,
                            world.players[0].faction,
                        )),
                        event.to.or(event.from),
                    )
                }
                EventKind::ResearchStarted
                | EventKind::ResearchCompleted
                | EventKind::ResearchCancelled
                | EventKind::UpgradeStarted
                | EventKind::UpgradeCompleted
                | EventKind::UpgradeCancelled
                    if event.player == Some(0) =>
                {
                    self.log.record(
                        event.tick,
                        DockLogKind::Research,
                        safe_detail(&research_words(event)),
                        event.to.or(event.from),
                    )
                }
                EventKind::Death if visible_death(world, event) => self.log.record(
                    event.tick,
                    DockLogKind::Death,
                    safe_detail(&death_words_in(world, event)),
                    event.to.or(event.from),
                ),
                // A seat going out is public: its buildings all fall.
                EventKind::Eliminated => {
                    if let Some(seat) = event.player {
                        self.log.record(
                            event.tick,
                            DockLogKind::Out,
                            safe_detail(&out_words(world, seat)),
                            None,
                        )
                    }
                }
                EventKind::Victory | EventKind::Draw => {
                    self.outcome_text = event.text.clone();
                    self.log.record(
                        event.tick,
                        DockLogKind::Outcome,
                        safe_detail(&outcome_words(world, event)),
                        None,
                    )
                }
                _ => {}
            }
        }
        self.record_hold_counts(world);
    }

    /// A count starting and running out are the moments a tide match turns
    /// on; both gauges are public, so both sides' counts are logged.
    fn record_hold_counts(&mut self, world: &World) {
        let before = self
            .last_hold
            .take()
            .unwrap_or_else(|| world.lane_hold.clone());
        self.last_hold = Some(world.lane_hold.clone());
        self.hold_peak.resize(world.lane_hold.len(), 0);
        for (player, (was, &now)) in before.into_iter().zip(&world.lane_hold).enumerate() {
            let seat = player as u8;
            if was == 0 && now > 0 {
                self.hold_peak[player] = now;
                self.log
                    .record(world.tick, DockLogKind::Hold, hold_words(world, seat), None);
            } else if was > 0 && now == 0 {
                // A seat knocked out loses its count with everything else;
                // its OUT row says so.
                if world.is_eliminated(seat) {
                    continue;
                }
                // The row's own kind already says COUNT, and "ran out at
                // 2s" read both as a count down to 2s and as one that got
                // to 2s (trial 10): the count emptied, and this was its
                // best.
                let peak = self.hold_peak[player].div_ceil(30);
                self.log.record(
                    world.tick,
                    DockLogKind::Hold,
                    format!(
                        "{} EMPTIED, PEAK {}S OF {}S",
                        whose_count(world, seat),
                        peak,
                        world.hold_ticks() / 30
                    ),
                    None,
                );
            } else {
                self.hold_peak[player] = self.hold_peak[player].max(now);
            }
        }
    }
}

fn find_record<'a>(world: &'a World, pending: &PendingCommand) -> Option<&'a CommandRecord> {
    world
        .command_log
        .iter()
        .find(|record| record.player == pending.player && record.sequence == pending.sequence)
}

fn visible_death(world: &World, event: &Event) -> bool {
    event.player == Some(0)
        || event
            .from
            .or(event.to)
            .is_some_and(|pos| world.visible(0, pos))
}

/// Whose count, standing alone: "YOURS", "ENEMY'S", "VIOLET'S".
fn whose_count(world: &World, seat: u8) -> String {
    match crate::seats::seat_owner_word(world, seat) {
        "YOUR" => "YOURS".to_string(),
        word if word.ends_with("'S") => word.to_string(),
        word => format!("{word}'S"),
    }
}

/// A research or upgrade line says which end it marks: the simulation's
/// upgrade events carry the name alone, so "OVERHAUL I" stood twice in the
/// log, once started and once done (trial 12).
pub(crate) fn research_words(event: &Event) -> String {
    match event.kind {
        EventKind::UpgradeStarted => format!("{} STARTED", event.text),
        EventKind::UpgradeCompleted => format!("{} DONE", event.text),
        _ => event.text.clone(),
    }
}

/// A count starting: "YOU HOLD BOTH LANES", "ENEMY HOLDS BOTH
/// LANES"; with three seats each holds the two lanes beside it, "RED
/// HOLDS ITS LANES".
fn hold_words(world: &World, seat: u8) -> String {
    let name = crate::seats::seat_name(world, seat);
    match (seat, world.seat_count() > 2) {
        (0, false) => "YOU HOLD BOTH LANES".into(),
        (_, false) => format!("{name} HOLDS BOTH LANES"),
        (0, true) => "YOU HOLD YOUR LANES".into(),
        (_, true) => format!("{name} HOLDS ITS LANES"),
    }
}

/// The sluice changing hands: the simulation's words with two seats, who
/// took it with three.
fn claim_words(world: &World, event: &Event) -> String {
    match event.player.filter(|_| world.seat_count() > 2) {
        Some(0) => "YOU TOOK THE SLUICE".into(),
        Some(seat) => format!("{} TOOK THE SLUICE", crate::seats::seat_name(world, seat)),
        None => event.text.clone(),
    }
}

/// A seat knocked out: "VIOLET IS OUT, 3RD", "YOU ARE OUT, 3RD".
fn out_words(world: &World, seat: u8) -> String {
    let place = crate::match_summary::place_out(&world.eliminated, world.seat_count(), seat)
        .map(|place| format!(", {}", crate::match_summary::ordinal(place)))
        .unwrap_or_default();
    if seat == 0 {
        format!("YOU ARE OUT{place}")
    } else {
        format!("{} IS OUT{place}", crate::seats::seat_name(world, seat))
    }
}

/// The result row: the simulation's words with two seats; with three,
/// the winner by colour, as two seats may share a faction name.
fn outcome_words(world: &World, event: &Event) -> String {
    if world.seat_count() <= 2 {
        return event.text.clone();
    }
    let tide = event.text.contains("HOLDS THE TIDE");
    match world.outcome {
        Some(Outcome::Victory(0)) if tide => "YOU HOLD THE TIDE".into(),
        Some(Outcome::Victory(0)) => "YOU WIN".into(),
        Some(Outcome::Victory(winner)) => format!(
            "{} {}",
            crate::seats::seat_name(world, winner),
            if tide { "HOLDS THE TIDE" } else { "WINS" }
        ),
        _ => event.text.clone(),
    }
}

/// A death row with the world to hand: the two-seat words, or with three
/// seats who killed whom by colour, "VIOLET KILLED RED'S LOOM", "RED
/// KILLED YOUR SOUNDER", "YOU KILLED VIOLET'S HOOK".
pub(crate) fn death_words_in(world: &World, event: &Event) -> String {
    if world.seat_count() <= 2 {
        return death_words(event);
    }
    let Some(owner) = event.player else {
        return death_words(event);
    };
    let whose = crate::seats::seat_owner_word(world, owner);
    match crate::match_summary::killer_seat(world, event) {
        Some(killer) => format!(
            "{} KILLED {whose} {}",
            crate::seats::seat_name(world, killer),
            event.text
        ),
        None if owner == 0 => format!("LOST {}", event.text),
        None => format!("{whose} {} FELL", event.text),
    }
}

/// A death row says whose machine fell and, for an own loss, to what:
/// "LOST SOUNDER TO LOOM", "KILLED REEDGUARD".
pub(crate) fn death_words(event: &Event) -> String {
    match (event.player == Some(0), event.cause) {
        (true, Some(cause)) => format!("LOST {} TO {}", event.text, cause.name()),
        (true, None) => format!("LOST {}", event.text),
        (false, _) => format!("KILLED {}", event.text),
    }
}

fn safe_detail(text: &str) -> String {
    // The simulation's event text is already bounded, but keeping the dock
    // rows short prevents a future event from overflowing the 640px canvas.
    text.chars().take(42).collect()
}

/// Atlas key for the four outcome signals.  Practice exit has no `World`
/// outcome and intentionally returns `None`.
pub fn result_signal_key(outcome: Option<&Outcome>, surrendered: bool) -> Option<&'static str> {
    match outcome {
        Some(Outcome::Victory(_)) if surrendered => Some("result_signal_3"),
        Some(Outcome::Victory(0)) => Some("result_signal_0"),
        Some(Outcome::Victory(_)) => Some("result_signal_1"),
        Some(Outcome::Draw) => Some("result_signal_2"),
        None => None,
    }
}

/// Atlas key for a faction's completed doctrine plate.
pub fn doctrine_plate_key(faction: Faction, doctrine: Doctrine) -> &'static str {
    match (faction, doctrine) {
        (Faction::Union, Doctrine::Hauling) => "doctrine_union_hauling",
        (Faction::Union, Doctrine::FireControl) => "doctrine_union_fire_control",
        (Faction::Assembly, Doctrine::Hauling) => "doctrine_assembly_hauling",
        (Faction::Assembly, Doctrine::FireControl) => "doctrine_assembly_fire_control",
        // Art v23: the salt bin on stilts and the mirror mast.
        (Faction::Compact, Doctrine::Hauling) => "doctrine_compact_hauling",
        (Faction::Compact, Doctrine::FireControl) => "doctrine_compact_fire_control",
    }
}

/// Draw a telegraph with the root-authored 48×24 asset when available, or a
/// readable bounded fallback when an atlas is unavailable.  The asset's
/// authored anchor is (0, 0); callers pass the exact desired top-left point.
pub fn draw_telegraph(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    telegraph: &DockTelegraph,
    x: i32,
    y: i32,
) {
    if atlas.is_some_and(|atlas| atlas.draw(canvas, telegraph.asset_key(), x, y, false)) {
        return;
    }
    let color = match telegraph.status {
        TelegraphStatus::Idle => EDGE,
        TelegraphStatus::Pending => GOLD,
        TelegraphStatus::Accepted => JADE,
        TelegraphStatus::Rejected => RED,
    };
    canvas.rect(x, y, 48, 24, PANEL);
    canvas.frame(x, y, 48, 24, color);
    canvas.rect(x + 3, y + 4, 3, 16, color);
    canvas.text(telegraph.status.compact_label(), x + 9, y + 4, color);
    let caption = telegraph.caption.chars().take(5).collect::<String>();
    canvas.text(&caption, x + 9, y + 15, MUTED);
}

/// Draw a compact history chart over an existing world.  It paints only its
/// bounded panel; it never clears the canvas beneath it.
///
/// The timeline strip sits under the legend and the rows start below it, so
/// the two never share pixels. Rows are newest first, runs of deaths are
/// folded into one row, and `scroll` skips that many rows toward the start
/// of the match.
pub fn draw_chart(
    canvas: &mut Canvas,
    log: &DockLog,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scroll: usize,
) {
    if w < 36 || h < 28 {
        return;
    }
    canvas.rect(x, y, w, h, [28, 45, 54, 232]);
    canvas.frame(x, y, w, h, EDGE);
    canvas.text_readable("DOCK LOG", x + 8, y + 6, WHITE);
    let wide = w >= 180;
    if wide {
        canvas.text("GATE/HOLD", x + 8, y + 20, GOLD);
        canvas.text("BUILD/RESEARCH", x + 68, y + 20, JADE);
        canvas.text("DEATH", x + 156, y + 20, RED);
    }
    let entries: Vec<&DockLogEntry> = log
        .entries
        .iter()
        .filter(|entry| entry.kind.is_chart_major())
        .collect();
    if entries.is_empty() {
        canvas.text_readable(
            if log.history_available {
                "NO EVENTS"
            } else {
                "NO MATCH LOG"
            },
            x + 8,
            y + if wide { 32 } else { 23 },
            if log.history_available { MUTED } else { GOLD },
        );
        return;
    }
    // Place marks from their actual observed ticks.  Each mark has a fixed
    // height; the chart communicates timing and category, never an invented
    // magnitude derived from the tick number.
    let base_y = if wide { y + 40 } else { y + h - 12 };
    canvas.line(x + 8, base_y, x + w - 8, base_y, EDGE);
    let inner_w = (w - 16).max(1);
    let first = entries.first().map_or(0, |entry| entry.tick);
    let last = entries.last().map_or(first, |entry| entry.tick);
    let span = last.saturating_sub(first).max(1);
    for entry in &entries {
        let delta = entry.tick.saturating_sub(first).min(span);
        let offset =
            ((u128::from(delta) * inner_w as u128) / u128::from(span)).min(inner_w as u128) as i32;
        let px = x + 8 + offset;
        let color = entry.kind.chart_color();
        match entry.kind {
            DockLogKind::GateWarning | DockLogKind::GateSwitch | DockLogKind::GateClaim => {
                canvas.diamond(px, base_y - 4, 3, 4, color)
            }
            DockLogKind::Hold => canvas.rect(px - 2, base_y - 9, 5, 3, color),
            DockLogKind::Death => canvas.frame(px - 2, base_y - 7, 5, 7, color),
            DockLogKind::Outcome => canvas.rect(px - 1, base_y - 8, 3, 8, color),
            // A seat out: a cross.
            DockLogKind::Out => {
                canvas.line(px - 2, base_y - 7, px + 2, base_y - 3, color);
                canvas.line(px - 2, base_y - 3, px + 2, base_y - 7, color);
            }
            DockLogKind::Construction | DockLogKind::Research => {
                canvas.rect(px - 1, base_y - 6, 3, 6, color)
            }
        }
    }
    canvas.text(&format_tick(first), x + 8, base_y + 2, MUTED);
    let last_label = format_tick(last);
    let label_x = (x + w - 8 - last_label.len() as i32 * 6).max(x + 8);
    canvas.text(&last_label, label_x, base_y + 2, MUTED);
    if !wide {
        return;
    }
    let rows = log_rows(&entries);
    let row_start = base_y + 14;
    // Room in printed lines: a long row wraps under its own time rather
    // than running off the panel (trial 10's eleven-death row).
    let room = ((y + h - 4 - row_start) / 10).max(0) as usize;
    let max_chars = ((w - 16) / 6).max(1) as usize;
    let scroll = scroll.min(rows.len().saturating_sub(1));
    let mut line = 0;
    let mut shown = 0;
    for row in rows.iter().rev().skip(scroll) {
        let lines = wrap_row(&row.text, max_chars);
        let fits = line + lines.len() <= room;
        // The first row always shows, as much of it as the panel holds.
        if !fits && shown > 0 {
            break;
        }
        for text in lines.iter().take(room.saturating_sub(line)) {
            canvas.text(
                text,
                x + 8,
                row_start + line as i32 * 10,
                row.kind.chart_color(),
            );
            line += 1;
        }
        shown += 1;
        if !fits {
            break;
        }
    }
    let older = rows.len().saturating_sub(scroll + shown);
    if older > 0 {
        // Beside the NEWER and OLDER buttons, which sit at the title row's
        // right end.
        let note = format!("{older} OLDER");
        canvas.text(
            &note,
            x + w - 8 - LOG_SCROLL_BUTTONS_W - note.len() as i32 * 6,
            y + 6,
            MUTED,
        );
    }
}

/// Width the result screen keeps at the right of the log's title row for
/// its NEWER and OLDER buttons.
pub const LOG_SCROLL_BUTTONS_W: i32 = 148;

/// A log row in lines of at most `chars`, broken between words; the lines
/// after the first start under the row's text, clear of its time.
pub(crate) fn wrap_row(text: &str, chars: usize) -> Vec<String> {
    let chars = chars.max(8);
    let indent = text.find(' ').map_or(0, |at| at + 1).min(chars / 2);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let mut word = word.to_string();
        loop {
            let limit = if lines.is_empty() {
                chars
            } else {
                chars - indent
            };
            let len = line.chars().count();
            let needed = if len == 0 { 0 } else { len + 1 };
            if needed + word.chars().count() <= limit {
                if len > 0 {
                    line.push(' ');
                }
                line.push_str(&word);
                break;
            }
            if len > 0 {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            // A single word longer than a line is cut where the line ends.
            let head: String = word.chars().take(limit).collect();
            word = word.chars().skip(limit).collect();
            lines.push(head);
            if word.is_empty() {
                break;
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    for continued in lines.iter_mut().skip(1) {
        continued.insert_str(0, &" ".repeat(indent));
    }
    lines
}

/// How many rows the log page shows, for the caller's scroll bound.
pub fn log_row_count(log: &DockLog) -> usize {
    let entries: Vec<&DockLogEntry> = log
        .entries
        .iter()
        .filter(|entry| entry.kind.is_chart_major())
        .collect();
    log_rows(&entries).len()
}

/// One printed row of the result log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRow {
    pub tick: u64,
    pub kind: DockLogKind,
    pub text: String,
}

/// Rows in match order. Deaths within ten seconds of the first in a run are
/// one row, so a single fight does not fill the page.
pub fn log_rows(entries: &[&DockLogEntry]) -> Vec<LogRow> {
    let mut rows = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        if entry.kind != DockLogKind::Death {
            rows.push(LogRow {
                tick: entry.tick,
                kind: entry.kind,
                text: chart_row(entry),
            });
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < entries.len()
            && entries[end].kind == DockLogKind::Death
            && entries[end].tick.saturating_sub(entry.tick) <= 300
        {
            end += 1;
        }
        if end - index == 1 {
            rows.push(LogRow {
                tick: entry.tick,
                kind: entry.kind,
                text: chart_row(entry),
            });
        } else {
            // Same words counted once: "LOST HOOK TO LOOM (3), KILLED SKIPPER".
            let mut counted: Vec<(&str, u32)> = Vec::new();
            for death in &entries[index..end] {
                match counted.iter_mut().find(|(text, _)| *text == death.detail) {
                    Some((_, n)) => *n += 1,
                    None => counted.push((&death.detail, 1)),
                }
            }
            let words = counted
                .iter()
                .map(|(text, n)| {
                    if *n > 1 {
                        format!("{text} ({n})")
                    } else {
                        (*text).to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            rows.push(LogRow {
                tick: entry.tick,
                kind: DockLogKind::Death,
                text: format!(
                    "{} {} DEATHS {}",
                    format_tick(entry.tick),
                    end - index,
                    words
                ),
            });
        }
        index = end;
    }
    rows
}

fn format_tick(tick: u64) -> String {
    let seconds = tick / 30;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn chart_row(entry: &DockLogEntry) -> String {
    let kind = match entry.kind {
        DockLogKind::GateWarning => "WARNING",
        DockLogKind::GateSwitch => "SWITCH",
        DockLogKind::GateClaim => "CLAIM",
        DockLogKind::Construction => "BUILD",
        DockLogKind::Research => "RESEARCH",
        DockLogKind::Death => "DEATH",
        DockLogKind::Outcome => "RESULT",
        DockLogKind::Hold => "COUNT",
        DockLogKind::Out => "OUT",
    };
    if entry.kind == DockLogKind::Out {
        // "VIOLET IS OUT, 3RD" says its own kind.
        return format!("{} {}", format_tick(entry.tick), entry.detail);
    }
    format!("{} {} {}", format_tick(entry.tick), kind, entry.detail)
}

/// The result screen's two pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResultPage {
    /// Army, income, the sluice, the counts, and what decided the match.
    #[default]
    Summary,
    /// The observed events, newest first.
    Log,
}

/// Draw the result treatment over the world.  The caller remains responsible
/// for result/replay/save buttons and the page tabs, which keeps them in the
/// existing keyboard focus chain.  Result signals use the authored 32×48
/// asset anchor (16, 44) at the supplied world/canvas point.
pub struct ResultOverlayOptions<'a> {
    pub outcome: Option<&'a Outcome>,
    pub surrendered: bool,
    pub practice_complete: bool,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The match summary; `None` shows the log alone (practice review).
    pub summary: Option<&'a crate::match_summary::MatchSummary>,
    pub page: ResultPage,
    pub log_scroll: usize,
    /// Whose result this is: the seat's side, or `None` for an observer.
    pub you: Option<u8>,
}

/// Where the page tabs sit inside a result panel at (x, y, w).
pub fn result_tab_rects(x: i32, y: i32, w: i32) -> [[i32; 4]; 2] {
    [[x + w - 196, y + 8, 92, 18], [x + w - 100, y + 8, 92, 18]]
}

/// The result's title and subtitle: the cause of the decision, not always
/// a fallen headquarters.
pub(crate) fn result_words(
    outcome: Option<&Outcome>,
    surrendered: bool,
    tide: bool,
    practice_complete: bool,
) -> (&'static str, &'static str, crate::canvas::Color) {
    match outcome {
        None if practice_complete => ("PRACTICE COMPLETE", "ALL SEVEN STEPS DONE.", JADE),
        None => ("PRACTICE ENDED", "ENDED BEFORE THE LAST STEP.", GOLD),
        Some(Outcome::Victory(_)) if surrendered => ("SURRENDERED", "THE COMPUTER WINS.", RED),
        Some(Outcome::Victory(0)) if tide => {
            ("VICTORY", "YOU HELD BOTH LANES FOR 90 SECONDS.", JADE)
        }
        Some(Outcome::Victory(_)) if tide => {
            ("DEFEAT", "THE ENEMY HELD BOTH LANES FOR 90 SECONDS.", RED)
        }
        Some(Outcome::Victory(0)) => ("VICTORY", "THE ENEMY HEADQUARTERS FELL.", JADE),
        Some(Outcome::Victory(_)) => ("DEFEAT", "YOUR HEADQUARTERS FELL.", RED),
        Some(Outcome::Draw) => ("DRAW", "BOTH HEADQUARTERS FELL TOGETHER.", GOLD),
    }
}

pub fn draw_result_overlay(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    dock: &DockState,
    options: &ResultOverlayOptions<'_>,
) {
    let ResultOverlayOptions {
        outcome,
        surrendered,
        practice_complete,
        x,
        y,
        w,
        h,
        summary,
        page,
        log_scroll,
        you,
    } = *options;
    if w < 160 || h < 160 {
        return;
    }
    let tide = dock.outcome_text.contains("HOLDS THE TIDE") || summary.is_some_and(|s| s.tide);
    let (title, color) =
        result_headline(outcome, summary, you, surrendered, tide, practice_complete);
    // The reason, in one line: what decided it, as this seat sees it.
    let reason = match (summary, outcome) {
        // Three seats: who won and where this seat placed.
        (Some(summary), Some(outcome)) if summary.free_for_all() => {
            three_seat_words(summary, outcome, you, surrendered, tide).1
        }
        (Some(summary), _) if summary.outcome.is_some() => summary.decided_line_for(you),
        _ => match (you, outcome) {
            (None, Some(Outcome::Victory(_))) if !dock.outcome_text.is_empty() => {
                dock.outcome_text.clone()
            }
            _ => result_words(outcome, surrendered, tide, practice_complete)
                .1
                .to_string(),
        },
    };

    // The banner: the outcome across the whole width, in its colour.
    let screen_w = canvas.width() as i32;
    let banner_h = 66;
    let field = mix(color, INK, 6);
    canvas.rect(0, y, screen_w, banner_h, field);
    canvas.rect(0, y, screen_w, 2, color);
    canvas.rect(0, y + banner_h - 2, screen_w, 2, color);
    let title = title.as_str();
    let scale = if crate::canvas::text_readable_width(title) * 4 <= w - 120 {
        4
    } else {
        3
    };
    let tw = crate::canvas::text_readable_width(title) * scale;
    canvas.text_readable_scaled(title, (screen_w - tw) / 2, y + 8, color, scale);
    let mark = if surrendered {
        ReasonMark::Flag
    } else if tide {
        ReasonMark::Crossings
    } else {
        ReasonMark::Headquarters
    };
    let readable = crate::canvas::text_readable_width(&reason);
    let (reason_w, small) = if readable + 14 <= w - 96 {
        (readable, false)
    } else {
        (reason.len() as i32 * 6, true)
    };
    let rx = (screen_w - reason_w - 14) / 2;
    // The outcome's signal lamp stands at the left end of the banner, when
    // the reason leaves it room.
    if rx >= x + 64
        && let Some(key) = result_signal_key(outcome, surrendered)
    {
        let drawn = atlas.is_some_and(|atlas| atlas.draw(canvas, key, x + 40, y + 57, false));
        if !drawn {
            draw_result_signal_fallback(canvas, x + 24, y + 13, color);
        }
    }
    draw_reason_mark(canvas, mark, rx, y + 48, WHITE);
    if small {
        canvas.text(&reason, rx + 14, y + 50, WHITE);
    } else {
        canvas.text_readable(&reason, rx + 14, y + 48, WHITE);
    }

    // The body: charts and the table, or the log.
    let (by, bh) = (y + banner_h + 8, h - banner_h - 8);
    canvas.rect(x, by, w, bh, [28, 45, 54, 244]);
    canvas.frame(x, by, w, bh, EDGE);
    let three = summary.filter(|summary| summary.free_for_all());
    match summary {
        Some(summary) if page == ResultPage::Summary => {
            crate::match_summary::draw_summary_page(
                canvas,
                summary,
                you,
                x + 8,
                by + 10,
                w - 16,
                bh - 16,
            );
        }
        _ => {
            canvas.text_readable(&dock.log.history_label(), x + 8, by + 12, MUTED);
            // The log page has no table: the placings ride beside the
            // history label, a chip and a place per seat.
            if let Some(summary) = three {
                let label_w = crate::canvas::text_readable_width(&dock.log.history_label());
                // Clear of the page tabs at the row's right end.
                let tabs = result_tab_rects(x, by, w)[0][0];
                draw_placings(
                    canvas,
                    summary,
                    you,
                    x + 8 + label_w + 16,
                    by + 13,
                    tabs - 8,
                );
            }
            draw_chart(
                canvas,
                &dock.log,
                x + 8,
                by + 32,
                w - 16,
                bh - 38,
                log_scroll,
            );
        }
    }
}

/// The big word across the result banner and its colour: the winner's
/// seat colour, gold for a draw.
fn result_headline(
    outcome: Option<&Outcome>,
    summary: Option<&crate::match_summary::MatchSummary>,
    you: Option<u8>,
    surrendered: bool,
    tide: bool,
    practice_complete: bool,
) -> (String, Color) {
    let colour = |seat: u8| summary.map_or(crate::seats::preview_colour(seat), |s| s.colour(seat));
    let three = summary.is_some_and(|s| s.free_for_all());
    match (outcome, you) {
        (Some(Outcome::Victory(winner)), Some(me)) if *winner == me => {
            ("VICTORY".into(), colour(me))
        }
        (Some(Outcome::Victory(winner)), Some(_)) => ("DEFEAT".into(), colour(*winner)),
        (Some(Outcome::Draw), Some(me))
            if three && summary.and_then(|s| s.place_of(me)) != Some(1) =>
        {
            ("DEFEAT".into(), RED)
        }
        (Some(Outcome::Draw), _) => ("DRAW".into(), GOLD),
        (Some(Outcome::Victory(winner)), None) => {
            let name = match summary {
                Some(s) if s.free_for_all() => s.seat_word(*winner, None),
                Some(s) => {
                    let side = s.side_name(*winner);
                    side.rsplit(' ').next().unwrap_or(side)
                }
                None => "",
            };
            let title = if name.is_empty() {
                "MATCH OVER".to_string()
            } else {
                format!("{name} WINS")
            };
            (title, colour(*winner))
        }
        (None, _) => {
            let (title, _, color) = result_words(outcome, surrendered, tide, practice_complete);
            (title.into(), color)
        }
    }
}

/// The small mark before the result's reason.
#[derive(Clone, Copy)]
enum ReasonMark {
    Headquarters,
    Crossings,
    Flag,
}

fn draw_reason_mark(canvas: &mut Canvas, mark: ReasonMark, x: i32, y: i32, color: Color) {
    match mark {
        // A tower with a door.
        ReasonMark::Headquarters => {
            canvas.rect(x + 1, y + 3, 8, 6, color);
            canvas.rect(x + 3, y, 4, 3, color);
            canvas.rect(x + 4, y + 6, 2, 3, INK);
        }
        // Two crossings: two banks with water between.
        ReasonMark::Crossings => {
            canvas.rect(x, y + 1, 10, 2, color);
            canvas.rect(x, y + 6, 10, 2, color);
            canvas.rect(x + 2, y + 4, 2, 1, color);
            canvas.rect(x + 6, y + 4, 2, 1, color);
        }
        // A flag on a pole.
        ReasonMark::Flag => {
            canvas.rect(x + 1, y, 1, 9, color);
            canvas.rect(x + 2, y, 7, 5, color);
        }
    }
}

/// `a` blended toward `b`: `parts` of 8 of `b`.
fn mix(a: Color, b: Color, parts: u16) -> Color {
    let blend = |x: u8, y: u8| ((u16::from(x) * (8 - parts) + u16::from(y) * parts) / 8) as u8;
    [blend(a[0], b[0]), blend(a[1], b[1]), blend(a[2], b[2]), 255]
}

/// The title, line and colour of a three-seat result: who won and where
/// the seat at this screen placed, or for an observer who won.
pub(crate) fn three_seat_words(
    summary: &crate::match_summary::MatchSummary,
    outcome: &Outcome,
    you: Option<u8>,
    surrendered: bool,
    tide: bool,
) -> (&'static str, String, Color) {
    use crate::match_summary::ordinal;
    let seat_word = |seat: u8, you: Option<u8>| summary.seat_word(seat, you);
    let seats = summary.seat_count();
    let placed = |seat: u8| {
        summary
            .place_of(seat)
            .map(|place| {
                if summary.tied(seat) {
                    format!("YOU TIED FOR {} OF {seats}.", ordinal(place))
                } else {
                    format!("YOU PLACED {} OF {seats}.", ordinal(place))
                }
            })
            .unwrap_or_default()
    };
    match (outcome, you) {
        (Outcome::Victory(winner), Some(me)) if *winner == me => {
            let others = (0..seats as u8)
                .filter(|&seat| seat != me)
                .map(|seat| seat_word(seat, you))
                .collect::<Vec<_>>()
                .join(" AND ");
            let line = if tide {
                // The length the match ended on (rules 20): 75 s while
                // three stood, 120 s once a seat was out.
                let at = summary.outcome.as_ref().map_or(u64::MAX, |(tick, _)| *tick);
                format!(
                    "YOU HELD YOUR LANES FOR {} SECONDS.",
                    summary.hold_ticks_at(at) / 30
                )
            } else {
                format!("YOU OUTLASTED {others}.")
            };
            ("VICTORY", line, JADE)
        }
        (Outcome::Victory(winner), Some(me)) => {
            let wins = if tide { "HOLDS THE TIDE" } else { "WINS" };
            let line = format!("{} {wins}. {}", seat_word(*winner, you), placed(me));
            (
                if surrendered { "SURRENDERED" } else { "DEFEAT" },
                line,
                RED,
            )
        }
        (Outcome::Draw, Some(me)) if summary.place_of(me) == Some(1) => {
            ("DRAW", "THE LAST HEADQUARTERS FELL TOGETHER.".into(), GOLD)
        }
        (Outcome::Draw, Some(me)) => (
            "DEFEAT",
            format!("THE LAST TWO FELL TOGETHER. {}", placed(me)),
            RED,
        ),
        (Outcome::Victory(winner), None) => (
            "MATCH OVER",
            format!(
                "{} ({}) {}.",
                seat_word(*winner, None),
                summary.side_name(*winner),
                if tide { "HOLDS THE TIDE" } else { "WINS" }
            ),
            WHITE,
        ),
        (Outcome::Draw, None) => ("DRAW", "THE LAST HEADQUARTERS FELL TOGETHER.".into(), GOLD),
    }
}

/// One chip and place per seat, best first: "1ST VIOLET  2ND YOU  3RD RED".
fn draw_placings(
    canvas: &mut Canvas,
    summary: &crate::match_summary::MatchSummary,
    you: Option<u8>,
    x: i32,
    y: i32,
    right: i32,
) {
    // Each seat's chip and place, with its name while every name fits
    // before `right`: the chip's colour names the seat otherwise.
    let placings = summary.placings();
    let words = |named: bool| -> Vec<String> {
        placings
            .iter()
            .map(|&(seat, place)| {
                let place = summary
                    .place_word(seat)
                    .unwrap_or_else(|| crate::match_summary::ordinal(place));
                if named {
                    format!("{place} {}", summary.seat_word(seat, you))
                } else {
                    place
                }
            })
            .collect()
    };
    let width = |words: &[String]| -> i32 {
        words
            .iter()
            .map(|word| 8 + word.len() as i32 * 6 + 12)
            .sum::<i32>()
            - 12
    };
    let named = words(true);
    let words = if x + width(&named) <= right {
        named
    } else {
        words(false)
    };
    let mut px = x;
    for (&(seat, _), word) in placings.iter().zip(&words) {
        let colour = summary.colour(seat);
        canvas.rect(px, y + 1, 5, 5, colour);
        canvas.text(word, px + 8, y, colour);
        px += 8 + word.len() as i32 * 6 + 12;
    }
}

fn draw_result_signal_fallback(canvas: &mut Canvas, x: i32, y: i32, color: Color) {
    canvas.rect(x, y, 32, 48, INK);
    canvas.frame(x, y, 32, 48, color);
    canvas.diamond(x + 16, y + 22, 9, 12, color);
    canvas.rect(x + 7, y + 39, 18, 2, color);
}

/// Draw a doctrine plate at its exact authored top-left anchor (0, 0), with a
/// text fallback for builds that do not yet contain the new plate assets.
pub fn draw_doctrine_plate(
    canvas: &mut Canvas,
    atlas: Option<&Atlas>,
    faction: Faction,
    doctrine: Doctrine,
    x: i32,
    y: i32,
) {
    let key = doctrine_plate_key(faction, doctrine);
    if atlas.is_some_and(|atlas| atlas.draw(canvas, key, x, y, false)) {
        return;
    }
    canvas.rect(x, y, 32, 24, PANEL);
    canvas.frame(
        x,
        y,
        32,
        24,
        match faction {
            Faction::Union => GOLD,
            Faction::Assembly => JADE,
            Faction::Compact => COBALT,
        },
    );
    let label = match doctrine {
        Doctrine::Hauling => "HAUL",
        Doctrine::FireControl => "FIRE",
    };
    canvas.text(label, x + 2, y + 8, WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_core::Faction;

    #[test]
    fn telegraph_acceptance_waits_for_applied_authoritative_record() {
        let mut state = DockState::default();
        state.on_command_submitted(0, 1, "MOVE", Some(Pos::cell(10, 10)));
        let mut world = World::new(42, Faction::Union);
        world.ai_enabled = false;
        assert_eq!(state.telegraph.status, TelegraphStatus::Pending);
        state.after_authoritative_tick(&world);
        assert_eq!(state.telegraph.status, TelegraphStatus::Pending);

        world.command_log.push(CommandRecord {
            tick: 1,
            player: 0,
            sequence: 1,
            command: bw_sim::Command::SwitchGate,
            accepted: true,
            applied: Some(true),
            reason: None,
        });
        world.tick = 1;
        state.after_authoritative_tick(&world);
        assert_eq!(state.telegraph.status, TelegraphStatus::Accepted);
        assert_eq!(state.telegraph.acknowledged_tick, Some(1));
    }

    #[test]
    fn rejected_record_carries_authoritative_reason() {
        let mut state = DockState::default();
        state.on_command_submitted(4, 7, "SWITCH", None);
        let mut world = World::new(7, Faction::Union);
        world.command_log.push(CommandRecord {
            tick: 5,
            player: 0,
            sequence: 7,
            command: bw_sim::Command::SwitchGate,
            accepted: true,
            applied: Some(false),
            reason: Some("gate is locked".into()),
        });
        world.tick = 5;
        state.after_authoritative_tick(&world);
        assert_eq!(state.telegraph.status, TelegraphStatus::Rejected);
        assert_eq!(state.telegraph.reason.as_deref(), Some("gate is locked"));
    }

    #[test]
    fn newer_immediate_rejection_wins_over_an_older_pending_ack() {
        let mut state = DockState::default();
        state.on_command_submitted(0, 1, "MOVE", Some(Pos::cell(10, 10)));
        state.on_command_rejected(0, None, "SWITCH", "gate is locked", None);

        let mut world = World::new(8, Faction::Union);
        world.command_log.push(CommandRecord {
            tick: 1,
            player: 0,
            sequence: 1,
            command: bw_sim::Command::SwitchGate,
            accepted: true,
            applied: Some(true),
            reason: None,
        });
        world.tick = 1;
        state.after_authoritative_tick(&world);
        assert_eq!(state.telegraph.status, TelegraphStatus::Rejected);
        assert_eq!(state.telegraph.caption, "SWITCH");
    }

    #[test]
    fn serialized_pending_pointer_recovers_against_authoritative_world() {
        let mut state = DockState::default();
        state.on_command_submitted(0, 1, "MOVE", Some(Pos::cell(10, 10)));
        let encoded = serde_json::to_vec(&state).expect("dock state serializes");
        let mut recovered: DockState = serde_json::from_slice(&encoded).expect("dock state loads");

        let mut world = World::new(12, Faction::Union);
        world.command_log.push(CommandRecord {
            tick: 1,
            player: 0,
            sequence: 1,
            command: bw_sim::Command::SwitchGate,
            accepted: true,
            applied: None,
            reason: None,
        });
        world.tick = 1;
        recovered.recover_from_world(&world);
        assert_eq!(recovered.telegraph.status, TelegraphStatus::Pending);

        world.command_log[0].applied = Some(true);
        recovered.after_authoritative_tick(&world);
        assert_eq!(recovered.telegraph.status, TelegraphStatus::Accepted);
    }

    #[test]
    fn missing_history_stays_partial_after_later_observations() {
        let mut state = DockState::default();
        state.mark_missing_history(12);
        let mut world = World::new(13, Faction::Union);
        world.ai_enabled = false;
        world.tick = 13;
        state.after_authoritative_tick(&world);
        assert!(state.log.history_label().contains("LOG SINCE LOAD"));

        world.events.push(bw_sim::Event {
            tick: 13,
            kind: EventKind::GateChanged,
            player: None,
            entity: None,
            other: None,
            from: None,
            to: Some(world.map.gate_pos),
            amount: 0,
            text: "north dry".into(),
            cause: None,
        });
        world.tick = 14;
        state.after_authoritative_tick(&world);
        assert_eq!(state.log.entries.len(), 1);
        assert!(state.log.history_label().contains("LOG SINCE LOAD"));
        assert!(state.log.history_label().ends_with("1 RECENT EVENT"));
        assert_eq!(state.log.history_start_tick, Some(12));
    }

    #[test]
    fn log_is_bounded_and_missing_history_is_explicit() {
        let mut log = DockLog::default();
        assert!(log.history_label().contains("NO MATCH LOG"));
        for tick in 0..(MAX_DOCK_LOG_ENTRIES + 7) {
            log.record(tick as u64, DockLogKind::GateSwitch, "switch", None);
        }
        assert_eq!(log.entries.len(), MAX_DOCK_LOG_ENTRIES);
        assert_eq!(log.entries[0].tick, 7);
        assert!(log.history_label().ends_with("EVENTS"));

        let encoded = serde_json::to_vec(&serde_json::json!({
            "entries": (0..(MAX_DOCK_LOG_ENTRIES + 3))
                .map(|tick| DockLogEntry::new(tick as u64, DockLogKind::Death, "visible", None))
                .collect::<Vec<_>>(),
            "history_available": true,
        }))
        .expect("oversized history serializes");
        let decoded: DockLog = serde_json::from_slice(&encoded).expect("history loads");
        assert_eq!(decoded.entries.len(), MAX_DOCK_LOG_DEATHS);
        assert_eq!(
            decoded.entries[0].tick,
            (MAX_DOCK_LOG_ENTRIES + 3 - MAX_DOCK_LOG_DEATHS) as u64
        );
    }

    #[test]
    fn deaths_never_push_the_sluice_story_out_of_the_log() {
        let mut log = DockLog::default();
        log.record(10, DockLogKind::GateClaim, "gate captured", None);
        log.record(20, DockLogKind::Hold, "ENEMY HOLDS BOTH LANES", None);
        for tick in 0..(MAX_DOCK_LOG_ENTRIES as u64 * 2) {
            log.record(100 + tick, DockLogKind::Death, "LOST HOOK TO LOOM", None);
        }
        assert_eq!(log.entries[0].kind, DockLogKind::GateClaim);
        assert_eq!(log.entries[1].kind, DockLogKind::Hold);
        assert_eq!(log.entries.len(), MAX_DOCK_LOG_DEATHS + 2);
        assert!(log.recent_only);
    }

    #[test]
    fn a_fight_is_one_row_and_counts_are_logged() {
        let entries = [
            DockLogEntry::new(30, DockLogKind::Death, "LOST HOOK TO LOOM", None),
            DockLogEntry::new(60, DockLogKind::Death, "LOST HOOK TO LOOM", None),
            DockLogEntry::new(90, DockLogKind::Death, "KILLED SKIPPER", None),
            DockLogEntry::new(900, DockLogKind::Death, "KILLED LOOM", None),
        ];
        let refs: Vec<&DockLogEntry> = entries.iter().collect();
        let rows = log_rows(&refs);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].text,
            "0:01 3 DEATHS LOST HOOK TO LOOM (2), KILLED SKIPPER"
        );
        assert_eq!(rows[1].text, "0:30 DEATH KILLED LOOM");

        let mut state = DockState::default();
        let mut world = World::new(9, Faction::Union);
        world.ai_enabled = false;
        for (tick, gauge) in [(1, 0), (2, 30), (3, 90), (4, 0)] {
            world.tick = tick;
            world.lane_hold = vec![0, gauge];
            state.after_authoritative_tick(&world);
        }
        let holds: Vec<&str> = state
            .log
            .entries
            .iter()
            .filter(|entry| entry.kind == DockLogKind::Hold)
            .map(|entry| entry.detail.as_str())
            .collect();
        assert_eq!(
            holds,
            ["ENEMY HOLDS BOTH LANES", "ENEMY'S EMPTIED, PEAK 3S OF 90S"]
        );
    }

    #[test]
    fn a_long_log_row_wraps_under_its_time_instead_of_running_off() {
        let row = "35:12 11 DEATHS VIOLET KILLED YOUR LOOM, YOU KILLED VIOLET'S BULWARK (2), YOU KILLED VIOLET'S SOUNDER";
        let lines = wrap_row(row, 40);
        assert!(lines.len() > 1);
        assert!(
            lines.iter().all(|line| line.chars().count() <= 40),
            "{lines:?}"
        );
        assert!(lines[0].starts_with("35:12 "));
        for line in &lines[1..] {
            assert!(
                line.starts_with("      ") && !line.starts_with("       "),
                "{line:?}"
            );
        }
        let words: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
        assert_eq!(
            words,
            row.split_whitespace().collect::<Vec<_>>(),
            "no word is lost"
        );
        assert_eq!(
            wrap_row("0:30 DEATH KILLED LOOM", 40),
            ["0:30 DEATH KILLED LOOM"]
        );
    }

    #[test]
    fn hidden_enemy_events_are_not_recorded() {
        let mut state = DockState::default();
        let mut world = World::new(9, Faction::Union);
        world.ai_enabled = false;
        world.events.push(bw_sim::Event {
            tick: 0,
            kind: EventKind::BuildStarted,
            player: Some(1),
            entity: Some(99),
            other: None,
            from: Some(Pos::cell(100, 100)),
            to: None,
            amount: 0,
            text: "Tower".into(),
            cause: None,
        });
        world.tick = 1;
        state.after_authoritative_tick(&world);
        assert!(state.log.entries.is_empty());
    }

    fn confluence() -> World {
        let mut world = World::with_map(
            4,
            bw_sim::MapId::Confluence,
            // Each faction wears its own colour: red, jade, violet.
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .expect("confluence");
        world.ai_enabled = false;
        world
    }

    fn fell(world: &mut World, seat: u8) {
        for entity in &mut world.entities {
            if entity.owner == seat && entity.kind == bw_core::Kind::Headquarters {
                entity.hp = 0;
            }
        }
        world.entities.retain(|entity| entity.hp > 0);
    }

    fn details(state: &DockState, kind: DockLogKind) -> Vec<String> {
        state
            .log
            .entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.detail.clone())
            .collect()
    }

    #[test]
    fn three_seats_are_named_by_colour_in_the_log() {
        let mut state = DockState::default();
        let mut world = confluence();
        for (tick, gauge) in [(1, 0), (2, 30), (3, 90), (4, 0)] {
            world.tick = tick;
            world.lane_hold = vec![0, 0, gauge];
            state.after_authoritative_tick(&world);
        }
        assert_eq!(
            details(&state, DockLogKind::Hold),
            ["VIOLET HOLDS ITS LANES", "VIOLET'S EMPTIED, PEAK 3S OF 75S"]
        );
        world.lane_hold = vec![0, 0, 0];
        // Violet falls, then the person: each is out with its place.
        fell(&mut world, 2);
        world.step();
        state.after_authoritative_tick(&world);
        fell(&mut world, 0);
        world.step();
        state.after_authoritative_tick(&world);
        assert_eq!(world.outcome, Some(Outcome::Victory(1)));
        assert_eq!(details(&state, DockLogKind::Out), ["VIOLET IS OUT, 3RD"]);
        assert_eq!(details(&state, DockLogKind::Outcome), ["JADE WINS"]);
        let row = log_rows(&state.log.entries.iter().collect::<Vec<_>>())
            .into_iter()
            .find(|row| row.kind == DockLogKind::Out)
            .expect("an out row");
        assert!(row.text.ends_with(" VIOLET IS OUT, 3RD"), "{}", row.text);
        assert!(!row.text.contains("OUT VIOLET"), "{}", row.text);
    }

    #[test]
    fn the_person_going_out_is_logged_with_the_place() {
        let mut state = DockState::default();
        let mut world = confluence();
        fell(&mut world, 0);
        world.step();
        state.after_authoritative_tick(&world);
        assert!(world.outcome.is_none());
        assert_eq!(details(&state, DockLogKind::Out), ["YOU ARE OUT, 3RD"]);
    }

    #[test]
    fn three_seat_deaths_say_who_killed_whom() {
        let world = confluence();
        let machine = |owner: u8| {
            world
                .entities
                .iter()
                .find(|e| e.owner == owner && !e.kind.is_building())
                .map(|e| e.id)
                .expect("a machine")
        };
        let death = |owner: u8, killer: Option<u32>| Event {
            tick: 0,
            kind: EventKind::Death,
            player: Some(owner),
            entity: Some(9999),
            other: killer,
            from: None,
            to: None,
            amount: 0,
            text: "LOOM".into(),
            cause: Some(bw_core::Kind::Hook),
        };
        assert_eq!(
            death_words_in(&world, &death(1, Some(machine(2)))),
            "VIOLET KILLED JADE'S LOOM"
        );
        assert_eq!(
            death_words_in(&world, &death(0, Some(machine(1)))),
            "JADE KILLED YOUR LOOM"
        );
        assert_eq!(
            death_words_in(&world, &death(2, Some(machine(0)))),
            "YOU KILLED VIOLET'S LOOM"
        );
        assert_eq!(
            death_words_in(&world, &death(2, None)),
            "VIOLET'S LOOM FELL"
        );
        // Two seats keep their words.
        let basin = World::new(1, Faction::Union);
        assert_eq!(
            death_words_in(&basin, &death(0, Some(machine(1)))),
            "LOST LOOM TO HOOK"
        );
    }

    #[test]
    fn a_three_seat_result_says_who_won_and_where_you_placed() {
        use crate::match_summary::MatchSummary;
        let mut summary = MatchSummary::default();
        summary.factions = vec![
            Some(Faction::Union),
            Some(Faction::Assembly),
            Some(Faction::Compact),
        ];
        summary.eliminated = vec![(0, 900)];
        summary.outcome = Some((1800, Outcome::Victory(1)));
        let winner = Outcome::Victory(1);
        assert_eq!(
            three_seat_words(&summary, &winner, Some(0), false, false),
            ("DEFEAT", "JADE WINS. YOU PLACED 3RD OF 3.".to_string(), RED)
        );
        assert_eq!(
            three_seat_words(&summary, &winner, Some(0), false, true).1,
            "JADE HOLDS THE TIDE. YOU PLACED 3RD OF 3."
        );
        assert_eq!(
            three_seat_words(&summary, &winner, None, false, false).1,
            "JADE (SILT ASSEMBLY) WINS."
        );
        summary.eliminated = vec![(2, 900)];
        summary.outcome = Some((1800, Outcome::Victory(0)));
        assert_eq!(
            three_seat_words(&summary, &Outcome::Victory(0), Some(0), false, false),
            (
                "VICTORY",
                "YOU OUTLASTED JADE AND VIOLET.".to_string(),
                JADE
            )
        );
    }

    #[test]
    fn outcome_signal_keys_cover_public_result_states() {
        assert_eq!(
            result_signal_key(Some(&Outcome::Victory(0)), false),
            Some("result_signal_0")
        );
        assert_eq!(
            result_signal_key(Some(&Outcome::Victory(1)), false),
            Some("result_signal_1")
        );
        assert_eq!(
            result_signal_key(Some(&Outcome::Draw), false),
            Some("result_signal_2")
        );
        assert_eq!(
            result_signal_key(Some(&Outcome::Victory(0)), true),
            Some("result_signal_3")
        );
        assert_eq!(result_signal_key(None, false), None);
    }
}
