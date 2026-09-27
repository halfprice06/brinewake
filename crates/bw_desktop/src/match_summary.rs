//! The match summary: what a finished match looked like over time, so the
//! result screen can say why it ended the way it did.
//!
//! Sampled from the authoritative world after every tick, in a live match, a
//! lockstep seat and a replay alike. The match is over when it is shown, so
//! it reads the whole world, not one seat's fog: every seat's army value,
//! income and crew, who owned the sluice, every hold count, every loss by
//! kind and by what killed it, and with three seats the order they went out
//! in. Presentation only: never hashed, never saved.

use crate::canvas::{Canvas, Color, EDGE, GOLD, MUTED, WHITE};
use bw_content::spec;
use bw_core::{Kind, Pos};
use bw_sim::{EventKind, Outcome, World};
use std::collections::{BTreeMap, VecDeque};

/// One sample every five seconds of match time.
pub const SAMPLE_EVERY: u64 = 150;
/// Income is the salvage earned over the last minute.
const INCOME_WINDOW: u64 = 1800;
/// A turning point is the most one side lost inside this window.
const TURN_WINDOW: u64 = 1800;

/// Every value is per seat, in seat order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub tick: u64,
    /// Salvage plus pressure cost of every living machine that is not a
    /// worker.
    pub army: Vec<u32>,
    /// Salvage earned over the last minute.
    pub income: Vec<u32>,
    pub crew: Vec<u32>,
    pub sluice: Option<u8>,
    /// Each seat's hold gauge in ticks.
    pub hold: Vec<u32>,
}

/// A match's presentation history rebuilt from its recording, seat by seat:
/// what a seat's summary and dock log would hold had it watched every tick
/// from the first. A match carried on from a recording (`--resume`) starts
/// its fresh processes part way through; without this their result screen
/// covered only the stretch after the restart (trial 10: "FROM 21:50").
#[derive(Clone, Debug, Default)]
pub struct History {
    pub tick: u64,
    pub seed: u64,
    /// Per canonical seat, as that seat's screen names the seats.
    pub seats: Vec<(MatchSummary, crate::dock_log::DockState)>,
}

impl History {
    /// An empty history for a match about to be played through from its
    /// first tick.
    pub fn new(world: &World) -> Self {
        let dock = || {
            let mut dock = crate::dock_log::DockState::default();
            dock.reset_for_new_match();
            dock
        };
        Self {
            tick: world.tick,
            seed: world.seed,
            seats: world
                .seats()
                .map(|_| (MatchSummary::default(), dock()))
                .collect(),
        }
    }

    /// Take in one tick of the canonical world, for every seat: each sees
    /// it turned so that it is seat 0, as its own screen would have.
    pub fn observe(&mut self, world: &World) {
        self.tick = world.tick;
        self.seed = world.seed;
        for (seat, (summary, dock)) in self.seats.iter_mut().enumerate() {
            if seat == 0 {
                summary.observe(world);
                dock.after_authoritative_tick(world);
            } else {
                let view = world.relabeled_for(seat as u8);
                summary.observe(&view);
                dock.after_authoritative_tick(&view);
            }
        }
    }

    /// Play a recording through to its end, keeping every seat's history.
    pub fn play(player: &mut bw_sim::ReplayPlayer) -> Result<Self, String> {
        let mut history = Self::new(player.world());
        while player.step()? {
            history.observe(player.world());
        }
        Ok(history)
    }

    /// `seat`'s summary and dock log, when this history reached `world`.
    pub fn for_seat(
        &self,
        world: &World,
        seat: u8,
    ) -> Option<(MatchSummary, crate::dock_log::DockState)> {
        if self.tick != world.tick || self.seed != world.seed {
            return None;
        }
        self.seats.get(usize::from(seat)).cloned()
    }
}

/// A seat's value in a per-seat list, zero when the list is short.
pub fn of(values: &[u32], seat: impl Into<usize>) -> u32 {
    values.get(seat.into()).copied().unwrap_or(0)
}

/// One hold count, from the gauge leaving zero to it falling back to zero or
/// deciding the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldRun {
    pub player: u8,
    pub start: u64,
    pub end: Option<u64>,
    pub peak: u32,
    pub won: bool,
    /// Times the count was broken without emptying: each one drained it
    /// for a while and the holder took it up again from what was left.
    pub breaks: u32,
    /// Whether every lane the count needs was held at the last observation.
    pub holding: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loss {
    pub tick: u64,
    pub owner: u8,
    pub kind: Kind,
    pub cause: Option<Kind>,
    pub pos: Option<Pos>,
    /// The seat that made the kill, when the world could still tell.
    pub killer: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchSummary {
    /// The tick sampling began: zero for a whole match, later after a load
    /// or a seek.
    pub start_tick: Option<u64>,
    pub samples: Vec<Sample>,
    /// Sluice captures: tick and new owner.
    pub captures: Vec<(u64, u8)>,
    /// Tide switches and floods.
    pub switches: Vec<u64>,
    pub holds: Vec<HoldRun>,
    pub losses: Vec<Loss>,
    /// Each seat's faction, in seat order.
    pub factions: Vec<Option<bw_core::Faction>>,
    /// The sampled world's rotation (the local seat's number in the
    /// match), so a seat's colour follows its player as on the field.
    pub turn: u8,
    /// Seats knocked out of a match of three, in the order they went out,
    /// with the tick.
    pub eliminated: Vec<(u8, u64)>,
    pub outcome: Option<(u64, Outcome)>,
    /// The match was decided by the tide rather than a headquarters.
    pub tide: bool,
    /// The losing side surrendered (with three seats: the last one out).
    pub surrendered: bool,
    earned: VecDeque<(u64, Vec<u64>)>,
}

fn earned(world: &World) -> Vec<u64> {
    world
        .players
        .iter()
        .map(|player| u64::from(player.salvage) + player.salvage_spent)
        .collect()
}

/// Army value: the cost of every living machine that is not a worker, per
/// seat.
pub fn army_value(world: &World) -> Vec<u32> {
    let mut army = vec![0u32; world.seat_count()];
    for entity in &world.entities {
        if entity.hp <= 0
            || entity.build_remaining > 0
            || entity.kind.is_building()
            || entity.kind.is_worker()
        {
            continue;
        }
        let Some(value) = army.get_mut(usize::from(entity.owner)) else {
            continue;
        };
        let s = spec(entity.kind);
        *value += s.salvage + s.pressure;
    }
    army
}

/// The seat whose machine made a kill, when the world can still tell: the
/// killer if it stands, or this tick's blow from it if it fell too. Never
/// the dead machine's own seat (a sunk transport's riders name their own
/// transport as the cause).
pub(crate) fn killer_seat(world: &World, event: &bw_sim::Event) -> Option<u8> {
    let id = event.other?;
    let owner = world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| entity.owner)
        .or_else(|| {
            world
                .events
                .iter()
                .find(|e| e.kind == EventKind::Damage && e.other == Some(id))
                .and_then(|e| e.player)
        })?;
    (Some(owner) != event.player && usize::from(owner) < world.seat_count()).then_some(owner)
}

/// Where a seat that went out placed: one more than the seats that
/// outlasted it, so seats that went out together share a place. `None` for
/// a seat still standing.
pub fn place_out(eliminated: &[(u8, u64)], seats: usize, seat: u8) -> Option<u8> {
    let out_at = |s: u8| {
        eliminated
            .iter()
            .find(|(out, _)| *out == s)
            .map(|(_, t)| *t)
    };
    let at = out_at(seat)?;
    let outlasted = (0..seats as u8)
        .filter(|&other| other != seat && out_at(other).is_none_or(|t| t > at))
        .count();
    Some(1 + outlasted as u8)
}

/// 1ST, 2ND, 3RD.
pub fn ordinal(place: u8) -> String {
    let suffix = match place {
        1 => "ST",
        2 => "ND",
        3 => "RD",
        _ => "TH",
    };
    format!("{place}{suffix}")
}

fn kind_named(name: &str) -> Option<Kind> {
    use Kind::*;
    [
        Hook,
        Riveter,
        Bulwark,
        Sounder,
        Wick,
        Skipper,
        Reedguard,
        Loom,
        Tidewatch,
        Lampwright,
        Caulker,
        Tender,
        Dredger,
        Caisson,
        Barge,
        Lifter,
        Headquarters,
        Works,
        Dropoff,
        Condenser,
        Tower,
        Drydock,
        Palisade,
        // The Compact's: missing, its losses went uncounted (trial 10's
        // result read LOST 1 for a seat that lost dozens).
        Raker,
        Brander,
        Heliostat,
        Glinter,
        Stilt,
        Glazier,
        Salter,
        Pan,
    ]
    .into_iter()
    .find(|kind| kind.name() == name)
}

pub fn clock(tick: u64) -> String {
    let seconds = tick / 30;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn thousands(n: u32) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

impl MatchSummary {
    /// The hold length at a tick (rules 20): 90 s with two seats, 75 s
    /// while three stand, 120 s once a seat went out before it.
    pub fn hold_ticks_at(&self, tick: u64) -> u32 {
        let out = self.eliminated.iter().any(|&(_, at)| at <= tick);
        bw_sim::hold_ticks_for(self.factions.len(), out)
    }

    /// Take in one authoritative tick. Call after every step.
    pub fn observe(&mut self, world: &World) {
        if self.start_tick.is_none() {
            self.start_tick = Some(world.tick.saturating_sub(1));
        }
        if self.factions.len() != world.seat_count() {
            self.factions = world.players.iter().map(|p| Some(p.faction)).collect();
        }
        self.turn = world.view_turn;
        if self.eliminated != world.eliminated {
            self.eliminated = world.eliminated.clone();
        }
        for event in &world.events {
            match event.kind {
                EventKind::GateCaptured => {
                    if let Some(player) = event.player {
                        self.captures.push((event.tick, player));
                    }
                }
                EventKind::GateChanged => self.switches.push(event.tick),
                EventKind::Death => {
                    let (Some(owner), Some(kind)) = (event.player, kind_named(&event.text)) else {
                        continue;
                    };
                    self.losses.push(Loss {
                        tick: event.tick,
                        owner,
                        kind,
                        cause: event.cause,
                        pos: event.from.or(event.to),
                        killer: killer_seat(world, event),
                    });
                }
                EventKind::Victory | EventKind::Draw => {
                    self.tide = event.text.contains("HOLDS THE TIDE");
                }
                _ => {}
            }
        }
        self.observe_holds(world);
        self.earned.push_back((world.tick, earned(world)));
        while self
            .earned
            .front()
            .is_some_and(|(tick, _)| world.tick.saturating_sub(*tick) > INCOME_WINDOW)
        {
            self.earned.pop_front();
        }
        if let Some(outcome) = &world.outcome
            && self.outcome.is_none()
        {
            self.outcome = Some((world.tick, outcome.clone()));
            if let Outcome::Victory(winner) = outcome {
                // With two seats the other one lost. With three, a
                // runner-up that was knocked out rather than left standing
                // went out last and so ended the match, by giving up.
                let loser = if world.seat_count() <= 2 {
                    Some(1 - (*winner).min(1))
                } else {
                    self.placings()
                        .into_iter()
                        .find(|&(seat, place)| place == 2 && self.out_at(seat).is_some())
                        .map(|(seat, _)| seat)
                };
                self.surrendered = loser.is_some_and(|loser| {
                    world.command_log.iter().any(|record| {
                        record.player == loser
                            && record.applied == Some(true)
                            && matches!(record.command, bw_sim::Command::Surrender)
                    })
                });
            }
            self.sample(world);
            return;
        }
        if world.tick.is_multiple_of(SAMPLE_EVERY) {
            self.sample(world);
        }
    }

    fn observe_holds(&mut self, world: &World) {
        for player in world.seats() {
            let now = world
                .lane_hold
                .get(usize::from(player))
                .copied()
                .unwrap_or(0);
            let open = self
                .holds
                .iter()
                .rposition(|run| run.player == player && run.end.is_none());
            match open {
                None if now > 0 => self.holds.push(HoldRun {
                    player,
                    start: world.tick,
                    end: None,
                    peak: now,
                    won: false,
                    breaks: 0,
                    holding: true,
                }),
                Some(index) => {
                    let holding = world.holds_every_lane(player);
                    let run = &mut self.holds[index];
                    if run.holding && !holding && now > 0 {
                        run.breaks += 1;
                    }
                    run.holding = holding;
                    run.peak = run.peak.max(now);
                    if now == 0 {
                        run.end = Some(world.tick);
                    } else if now >= world.hold_ticks() {
                        run.end = Some(world.tick);
                        run.won = true;
                    }
                }
                None => {}
            }
        }
    }

    fn sample(&mut self, world: &World) {
        let now = earned(world);
        let then = self
            .earned
            .front()
            .map_or_else(|| now.clone(), |(_, total)| total.clone());
        let span = self
            .earned
            .front()
            .map_or(1, |(tick, _)| world.tick.saturating_sub(*tick).max(1));
        let income = now
            .iter()
            .enumerate()
            .map(|(p, total)| {
                let gained = total.saturating_sub(then.get(p).copied().unwrap_or(*total));
                (gained * INCOME_WINDOW / span.max(INCOME_WINDOW / 4)) as u32
            })
            .collect();
        self.samples.push(Sample {
            tick: world.tick,
            army: army_value(world),
            income,
            crew: world.players.iter().map(|p| p.crew).collect(),
            sluice: world.gate.owner,
            hold: world.lane_hold.clone(),
        });
    }

    pub fn latest(&self) -> Option<&Sample> {
        self.samples.last()
    }

    pub fn first_tick(&self) -> u64 {
        self.start_tick.unwrap_or(0)
    }

    pub fn last_tick(&self) -> u64 {
        self.outcome
            .as_ref()
            .map(|(tick, _)| *tick)
            .or_else(|| self.samples.last().map(|s| s.tick))
            .unwrap_or(0)
    }

    /// How many seats played: two until a world has been observed.
    pub fn seat_count(&self) -> usize {
        self.factions.len().max(2)
    }

    /// Whether this is a free-for-all of three or more seats.
    pub fn free_for_all(&self) -> bool {
        self.seat_count() > 2
    }

    pub fn side_name(&self, player: u8) -> &'static str {
        let last = self.factions.len().max(1) - 1;
        self.factions
            .get(usize::from(player).min(last))
            .copied()
            .flatten()
            .map(|f| f.name())
            .unwrap_or(if player == 0 { "SIDE ONE" } else { "SIDE TWO" })
    }

    /// Each seat's colour, following its player as the field does.
    pub fn hue(&self, seat: u8) -> crate::seats::Hue {
        let factions: Vec<bw_core::Faction> = self
            .factions
            .iter()
            .map(|f| f.unwrap_or_default())
            .collect();
        crate::seats::view_hues(&factions, self.turn)
            .get(usize::from(seat))
            .copied()
            .unwrap_or(match seat {
                0 => crate::seats::Hue::Jade,
                1 => crate::seats::Hue::Red,
                _ => crate::seats::Hue::Violet,
            })
    }

    /// A seat's word with three seats: YOU for the seat at this screen,
    /// else its colour, the same on every screen. With two seats the
    /// screens name factions instead.
    pub fn seat_word(&self, seat: u8, you: Option<u8>) -> &'static str {
        if you == Some(seat) {
            return "YOU";
        }
        if !self.free_for_all() {
            return if seat == 0 { "JADE" } else { "RED" };
        }
        self.hue(seat).word()
    }

    /// A seat's name in the summary's lines: its faction with two seats,
    /// its colour (or YOU) with three, where two may share a faction.
    pub fn seat_name(&self, seat: u8, you: Option<u8>) -> &'static str {
        if self.free_for_all() {
            self.seat_word(seat, you)
        } else {
            self.side_name(seat)
        }
    }

    /// The colour a seat is drawn in: its own seat colour, as on the
    /// minimap and its machines' markers.
    pub fn colour(&self, seat: u8) -> Color {
        if self.free_for_all() {
            self.hue(seat).colour()
        } else {
            crate::seats::preview_colour(seat)
        }
    }

    /// When a seat went out, if it did before the end.
    pub fn out_at(&self, seat: u8) -> Option<u64> {
        self.eliminated
            .iter()
            .find(|(out, _)| *out == seat)
            .map(|(_, tick)| *tick)
    }

    /// Where a seat finished, 1 for the winner: one more than the seats
    /// that outlasted it, so seats that went out together share a place.
    /// `None` while its match goes on and it still stands.
    pub fn place_of(&self, seat: u8) -> Option<u8> {
        if let Some(place) = place_out(&self.eliminated, self.seat_count(), seat) {
            return Some(place);
        }
        match self.outcome {
            Some((_, Outcome::Victory(winner))) => Some(if winner == seat { 1 } else { 2 }),
            Some((_, Outcome::Draw)) => Some(1),
            None => None,
        }
    }

    /// Every placed seat, best first: (seat, place).
    pub fn placings(&self) -> Vec<(u8, u8)> {
        let mut placed: Vec<(u8, u8)> = (0..self.seat_count() as u8)
            .filter_map(|seat| self.place_of(seat).map(|place| (seat, place)))
            .collect();
        placed.sort_by_key(|&(seat, place)| (place, seat));
        placed
    }

    /// Ticks each seat owned the sluice, from the samples.
    pub fn sluice_share(&self) -> Vec<u64> {
        let mut share = vec![0u64; self.seat_count()];
        let last = share.len() - 1;
        for pair in self.samples.windows(2) {
            if let Some(owner) = pair[0].sluice {
                share[usize::from(owner).min(last)] += pair[1].tick - pair[0].tick;
            }
        }
        share
    }

    /// Whether a loss counts as a seat's kill: with two seats every loss of
    /// the other side, with three only those the seat was seen to make.
    fn killed_by(&self, loss: &Loss, killer: u8) -> bool {
        if self.free_for_all() {
            loss.killer == Some(killer) && loss.owner != killer
        } else {
            loss.owner != killer
        }
    }

    /// Kills per side, by the kind that made them, most first.
    pub fn kills_by_kind(&self, killer: u8) -> Vec<(Kind, u32)> {
        let mut counts: BTreeMap<&'static str, (Kind, u32)> = BTreeMap::new();
        for loss in self.losses.iter().filter(|l| self.killed_by(l, killer)) {
            if let Some(cause) = loss.cause {
                counts.entry(cause.name()).or_insert((cause, 0)).1 += 1;
            }
        }
        let mut list: Vec<_> = counts.into_values().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.name().cmp(b.0.name())));
        list
    }

    /// Whether another seat finished in the same place as this one: the
    /// beaten seats of a match nobody was knocked out of, or seats that went
    /// out together.
    pub fn tied(&self, seat: u8) -> bool {
        self.place_of(seat).is_some_and(|place| {
            (0..self.seat_count() as u8)
                .any(|other| other != seat && self.place_of(other) == Some(place))
        })
    }

    /// A seat's place as the result shows it: "2ND", or "=2ND" when shared
    /// (trial 12: both losers read "2ND" and all three players asked why).
    pub fn place_word(&self, seat: u8) -> Option<String> {
        let place = self.place_of(seat)?;
        let word = ordinal(place);
        Some(if self.tied(seat) {
            format!("={word}")
        } else {
            word
        })
    }

    /// The minute in which one side lost the most machine value: the start
    /// tick, the side, how many machines, their value and the kind that
    /// killed most of them.  A beaten side's worst minute: the winner's own
    /// worst loss is no turning point (trial 12), and stands only when no
    /// beaten side lost three machines in a minute.
    pub fn turning_point(&self) -> Option<(u64, u8, u32, u32, Option<Kind>)> {
        let winner = match self.outcome {
            Some((_, Outcome::Victory(winner))) => Some(winner),
            _ => None,
        };
        self.worst_minute(|owner| Some(owner) != winner)
            .or_else(|| self.worst_minute(|_| true))
    }

    /// Whether the turning point is the winner's own loss: the result then
    /// calls it the heaviest loss, not the turn of the match.
    pub fn turning_point_is_winners(&self) -> bool {
        matches!(
            (self.turning_point(), &self.outcome),
            (Some((_, owner, ..)), Some((_, Outcome::Victory(winner)))) if owner == *winner
        )
    }

    /// The minute in which a side `among` allows lost the most machine
    /// value, of at least three machines.
    fn worst_minute(
        &self,
        among: impl Fn(u8) -> bool,
    ) -> Option<(u64, u8, u32, u32, Option<Kind>)> {
        let mut best: Option<(u64, u8, u32, u32, Option<Kind>)> = None;
        for owner in (0..self.seat_count() as u8).filter(|&owner| among(owner)) {
            let own: Vec<&Loss> = self
                .losses
                .iter()
                .filter(|l| l.owner == owner && !l.kind.is_building())
                .collect();
            let mut tail = 0;
            let mut value = 0u32;
            for head in 0..own.len() {
                value += cost(own[head].kind);
                while own[head].tick - own[tail].tick > TURN_WINDOW {
                    value -= cost(own[tail].kind);
                    tail += 1;
                }
                if best.is_none_or(|b| value > b.3) {
                    let window = &own[tail..=head];
                    let mut causes: BTreeMap<&'static str, (Kind, u32)> = BTreeMap::new();
                    for loss in window {
                        if let Some(cause) = loss.cause {
                            causes.entry(cause.name()).or_insert((cause, 0)).1 += 1;
                        }
                    }
                    let cause = causes
                        .into_values()
                        .max_by_key(|(_, n)| *n)
                        .map(|(kind, _)| kind);
                    best = Some((own[tail].tick, owner, window.len() as u32, value, cause));
                }
            }
        }
        best.filter(|b| b.2 >= 3)
    }

    /// One line on what decided the match, as the first seat sees it.
    #[cfg(test)]
    pub fn decided_line(&self) -> String {
        self.decided_line_for(Some(0))
    }

    /// One line on what decided the match; `you` names the seat at this
    /// screen with three seats (`None` for an observer).
    pub fn decided_line_for(&self, you: Option<u8>) -> String {
        let Some((tick, outcome)) = &self.outcome else {
            return "THE MATCH WAS NOT DECIDED.".into();
        };
        let three = self.free_for_all();
        let name = |seat: u8| self.seat_name(seat, you);
        match outcome {
            Outcome::Draw if three => {
                format!("THE LAST HEADQUARTERS FELL TOGETHER AT {}.", clock(*tick))
            }
            Outcome::Draw => format!("BOTH HEADQUARTERS FELL AT {}.", clock(*tick)),
            Outcome::Victory(winner) => {
                // The seat beaten last: the other side, or with three seats
                // the runner-up (the last one out, or one left standing).
                let loser = if three {
                    self.placings()
                        .into_iter()
                        .find(|&(_, place)| place == 2)
                        .map_or(1 - (*winner).min(1), |(seat, _)| seat)
                } else {
                    1 - (*winner).min(1)
                };
                if self.surrendered {
                    // With three seats, when it went out, as its block says.
                    let at = self.out_at(loser).filter(|_| three).unwrap_or(*tick);
                    let who = if you == Some(loser) {
                        "YOU"
                    } else {
                        name(loser)
                    };
                    return format!("{who} SURRENDERED AT {}.", clock(at));
                }
                let its = if three && you == Some(*winner) {
                    "YOUR"
                } else {
                    "ITS"
                };
                if self.tide {
                    let run = self.holds.iter().rev().find(|run| run.player == *winner);
                    let from = run
                        .map(|run| run.start)
                        .unwrap_or(tick.saturating_sub(u64::from(self.hold_ticks_at(*tick))));
                    let breaks = run.map_or(0, |run| run.breaks);
                    // A count broken and taken up again is not one unbroken
                    // hold: both ninth-trial seats read it that way.
                    if breaks > 0 {
                        format!(
                            "{} FILLED {its} COUNT FROM {} TO {}, BROKEN {} TIME{} BUT NEVER EMPTIED.",
                            name(*winner),
                            clock(from),
                            clock(*tick),
                            breaks,
                            if breaks == 1 { "" } else { "S" }
                        )
                    } else if three {
                        // A seat of three holds the two lanes beside it.
                        format!(
                            "{} HELD {its} LANES FROM {} TO {}.",
                            name(*winner),
                            clock(from),
                            clock(*tick)
                        )
                    } else {
                        format!(
                            "{} HELD BOTH LANES FROM {} TO {}.",
                            name(*winner),
                            clock(from),
                            clock(*tick)
                        )
                    }
                } else if three {
                    format!("{} WAS THE LAST TO FALL, AT {}.", name(loser), clock(*tick))
                } else {
                    format!("{} HEADQUARTERS FELL AT {}.", name(loser), clock(*tick))
                }
            }
        }
    }

    /// One line on the fight that turned the match, if there was one.
    #[cfg(test)]
    pub fn turning_line(&self) -> Option<String> {
        self.turning_line_for(Some(0))
    }

    /// The turning point, naming seats as `you` sees them.
    pub fn turning_line_for(&self, you: Option<u8>) -> Option<String> {
        let (tick, owner, count, value, cause) = self.turning_point()?;
        Some(format!(
            "{} {} LOST {} MACHINES (COST {}){}.",
            clock(tick),
            self.seat_name(owner, you),
            count,
            thousands(value),
            cause
                .map(|k| format!(" TO {}", plural(k)))
                .unwrap_or_default()
        ))
    }

    /// A seat's best hold count in ticks, and whether a count of its won.
    pub fn best_hold(&self, player: u8) -> (u32, bool) {
        let runs = self.holds.iter().filter(|r| r.player == player);
        let won = runs.clone().any(|r| r.won);
        (runs.map(|r| r.peak).max().unwrap_or(0), won)
    }
}

/// A chart of one sampled value, one line per seat.
fn draw_series(
    canvas: &mut Canvas,
    summary: &MatchSummary,
    label: &str,
    value: impl Fn(&Sample, usize) -> u32,
    area: [i32; 4],
) {
    let [x, y, w, h] = area;
    canvas.text(label, x, y, MUTED);
    let top = y + 9;
    let gh = h - 9;
    canvas.rect(x, top, w, gh, [23, 38, 46, 255]);
    canvas.frame(x, top, w, gh, EDGE);
    let samples = &summary.samples;
    let seats = summary.seat_count();
    let peak = samples
        .iter()
        .flat_map(|s| (0..seats).map(|p| value(s, p)).collect::<Vec<_>>())
        .max()
        .unwrap_or(0)
        .max(1);
    let peak_text = thousands(peak);
    canvas.text(&peak_text, x + w - 2 - peak_text.len() as i32 * 6, y, MUTED);
    if samples.len() < 2 {
        return;
    }
    let t0 = summary.first_tick();
    let span = summary.last_tick().saturating_sub(t0).max(1);
    let px = |tick: u64| x + 1 + ((tick.saturating_sub(t0)) * (w - 3) as u64 / span) as i32;
    let py = |v: u32| top + gh - 2 - (u64::from(v) * (gh - 4) as u64 / u64::from(peak)) as i32;
    // The first seat is drawn last, on top.
    for side in (0..seats).rev() {
        let color = summary.colour(side as u8);
        for pair in samples.windows(2) {
            canvas.line(
                px(pair[0].tick),
                py(value(&pair[0], side)),
                px(pair[1].tick),
                py(value(&pair[1], side)),
                color,
            );
        }
    }
}

/// The sluice owner over time, the tide switches above it and every hold
/// count below it, on the same time axis as the charts.
pub fn draw_sluice_strip(
    canvas: &mut Canvas,
    summary: &MatchSummary,
    area: [i32; 4],
    upto: Option<u64>,
) {
    let [x, y, w, h] = area;
    canvas.rect(x, y, w, h, [23, 38, 46, 255]);
    let t0 = summary.first_tick();
    let span = summary.last_tick().saturating_sub(t0).max(1);
    let px = |tick: u64| x + ((tick.saturating_sub(t0)).min(span) * w as u64 / span) as i32;
    let sluice_h = (h / 2).max(3);
    for pair in summary.samples.windows(2) {
        if let Some(owner) = pair[0].sluice {
            let a = px(pair[0].tick);
            let b = px(pair[1].tick).max(a + 1);
            canvas.rect(a, y, b - a, sluice_h, summary.colour(owner));
        }
    }
    for tick in &summary.switches {
        let sx = px(*tick);
        canvas.rect(sx, y, 1, sluice_h, WHITE);
    }
    let count_y = y + sluice_h + 1;
    let count_h = h - sluice_h - 1;
    for run in &summary.holds {
        let a = px(run.start);
        let b = px(run.end.unwrap_or(summary.last_tick())).max(a + 1);
        // A count is drawn as tall as it got toward the ninety seconds.
        let total = summary.hold_ticks_at(run.start);
        let filled =
            (i64::from(count_h) * i64::from(run.peak.min(total)) / i64::from(total)).max(1) as i32;
        canvas.rect(
            a,
            count_y + count_h - filled,
            b - a,
            filled,
            summary.colour(run.player),
        );
        if run.won {
            canvas.frame(a, count_y, b - a, count_h, WHITE);
        }
    }
    // With three seats a seat going out is a dark notch through the strip
    // with that seat's colour at its foot.
    if summary.free_for_all() {
        for &(seat, tick) in &summary.eliminated {
            let sx = px(tick);
            canvas.rect(sx, y, 1, h, crate::canvas::INK);
            canvas.rect(sx - 1, y + h - 3, 3, 3, summary.colour(seat));
        }
    }
    canvas.frame(x - 1, y - 1, w + 2, h + 2, EDGE);
    if let Some(tick) = upto {
        let sx = px(tick);
        canvas.rect(sx - 1, y - 2, 3, h + 4, WHITE);
    }
}

/// One table cell: the value it ranks by and what it shows.
type Cell = (u64, String);

/// A seat's short name over its column: YOU, its colour with three seats,
/// or its faction's last word.
fn column_name(summary: &MatchSummary, seat: u8, you: Option<u8>) -> &'static str {
    if summary.free_for_all() || you == Some(seat) {
        return summary.seat_word(seat, you);
    }
    let side = summary.side_name(seat);
    side.rsplit(' ').next().unwrap_or(side)
}

/// The summary page of the result screen: army value and income for every
/// seat over the match and the sluice and hold strip on the left; on the
/// right a table, one column per seat, and the turning point. With three
/// seats the columns run in placing order.
pub fn draw_summary_page(
    canvas: &mut Canvas,
    summary: &MatchSummary,
    you: Option<u8>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let chart_w = (w * 3 / 5).max(120);
    let partial = summary.first_tick() > 0;
    let three = summary.free_for_all();
    let seats = summary.seat_count() as u8;
    if partial {
        canvas.text(
            &format!("FROM {}", clock(summary.first_tick())),
            x + chart_w - 60,
            y,
            MUTED,
        );
    }
    let graph_h = ((h - 60) / 2).clamp(28, 60);
    draw_series(
        canvas,
        summary,
        "ARMY VALUE",
        |s, p| of(&s.army, p),
        [x, y + 12, chart_w, graph_h],
    );
    draw_series(
        canvas,
        summary,
        "INCOME PER MINUTE",
        |s, p| of(&s.income, p),
        [x, y + 16 + graph_h, chart_w, graph_h],
    );
    let strip_y = y + 20 + graph_h * 2;
    canvas.text("SLUICE / COUNTS", x, strip_y, MUTED);
    draw_sluice_strip(
        canvas,
        summary,
        [x + 1, strip_y + 10, chart_w - 2, 14],
        None,
    );
    let axis_y = strip_y + 27;
    canvas.text(&clock(summary.first_tick()), x, axis_y, MUTED);
    let end = clock(summary.last_tick());
    canvas.text(&end, x + chart_w - end.len() as i32 * 6, axis_y, MUTED);

    // The table: you first with two seats, placing order with three.
    let cx = x + chart_w + 12;
    let cw = w - chart_w - 12;
    let mut order: Vec<u8> = (0..seats).collect();
    if three {
        order.sort_by_key(|&seat| (summary.place_of(seat).unwrap_or(u8::MAX), seat));
    } else if let Some(me) = you {
        order.sort_by_key(|&seat| seat != me);
    }
    let label_w = 72;
    let col_w = (cw - label_w) / i32::from(seats.max(1));
    let col_right = |i: usize| cx + label_w + col_w * (i as i32 + 1) - 2;
    let mut ty = y + 22;
    if three {
        for (i, &seat) in order.iter().enumerate() {
            if let Some(word) = summary.place_word(seat) {
                canvas.text(&word, col_right(i) - word.len() as i32 * 6, ty, MUTED);
            }
        }
        ty += 10;
    }
    for (i, &seat) in order.iter().enumerate() {
        let name = column_name(summary, seat, you);
        let chars = ((col_w - 10) / 6).max(1) as usize;
        let name = fit(name, chars);
        let nx = col_right(i) - name.len() as i32 * 6;
        canvas.rect(nx - 8, ty + 1, 5, 5, summary.colour(seat));
        canvas.text(&name, nx, ty, summary.colour(seat));
    }
    ty += 11;
    canvas.line(cx, ty - 2, cx + cw - 1, ty - 2, EDGE);

    let share = summary.sluice_share();
    let whole = summary
        .last_tick()
        .saturating_sub(summary.first_tick())
        .max(1);
    let peak = |value: fn(&Sample) -> &Vec<u32>, seat: u8| {
        summary
            .samples
            .iter()
            .map(|s| of(value(s), seat))
            .max()
            .unwrap_or(0)
    };
    // Each row: its label, a value per seat to rank by, what is shown, and
    // whether more is better.
    let mut rows: Vec<(&str, Vec<Cell>, bool)> = Vec::new();
    let per_seat = |f: &dyn Fn(u8) -> Cell| order.iter().map(|&seat| f(seat)).collect();
    rows.push((
        "LOST",
        per_seat(&|seat| {
            let n = summary.losses.iter().filter(|l| l.owner == seat).count() as u64;
            (n, n.to_string())
        }),
        false,
    ));
    rows.push((
        "KILLED",
        per_seat(&|seat| {
            let n = summary
                .losses
                .iter()
                .filter(|l| summary.killed_by(l, seat))
                .count() as u64;
            (n, n.to_string())
        }),
        true,
    ));
    rows.push((
        "PEAK ARMY",
        per_seat(&|seat| {
            let n = peak(|s| &s.army, seat);
            (u64::from(n), thousands(n))
        }),
        true,
    ));
    rows.push((
        "PEAK INCOME",
        per_seat(&|seat| {
            let n = peak(|s| &s.income, seat);
            (u64::from(n), thousands(n))
        }),
        true,
    ));
    rows.push((
        "SLUICE HELD",
        per_seat(&|seat| {
            let n = share.get(usize::from(seat)).copied().unwrap_or(0) * 100 / whole;
            (n, format!("{n}%"))
        }),
        true,
    ));
    rows.push((
        "BEST HOLD",
        per_seat(&|seat| {
            let (best, won) = summary.best_hold(seat);
            let shown = if won {
                "WON".to_string()
            } else if best == 0 {
                "-".to_string()
            } else {
                format!("{}S", best / 30)
            };
            (u64::from(best) + if won { u64::MAX / 2 } else { 0 }, shown)
        }),
        true,
    ));
    if !three {
        // The kind that made the most kills: shown, not ranked.
        rows.push((
            "TOP UNIT",
            per_seat(&|seat| {
                let top = summary
                    .kills_by_kind(seat)
                    .first()
                    .map(|(kind, _)| kind.name());
                (0, top.unwrap_or("-").to_string())
            }),
            true,
        ));
    }
    for (label, values, more) in &rows {
        canvas.text(label, cx, ty, MUTED);
        // The row's leader is drawn in its colour; a tie has no leader.
        let best = if *more {
            values.iter().map(|v| v.0).max()
        } else {
            values.iter().map(|v| v.0).min()
        };
        let leaders = values.iter().filter(|v| Some(v.0) == best).count();
        for (i, ((rank, shown), &seat)) in values.iter().zip(&order).enumerate() {
            let out = three && summary.out_at(seat).is_some();
            let ink = if leaders == 1 && Some(*rank) == best {
                summary.colour(seat)
            } else if out {
                MUTED
            } else {
                WHITE
            };
            let shown = fit(shown, ((col_w - 2) / 6).max(1) as usize);
            canvas.text(&shown, col_right(i) - shown.len() as i32 * 6, ty, ink);
        }
        ty += 11;
    }
    canvas.line(cx, ty - 2, cx + cw - 1, ty - 2, EDGE);
    ty += 4;
    if let Some(turn) = summary.turning_line_for(you) {
        let chars = (cw / 6).max(8) as usize;
        let rows = ((y + h - ty - 10) / 9).clamp(0, 4) as usize;
        if rows > 0 {
            let label = if summary.turning_point_is_winners() {
                "HEAVIEST LOSS"
            } else {
                "TURNING POINT"
            };
            canvas.text(label, cx, ty, GOLD);
            ty += 10;
            for line in wrap(&turn, chars).into_iter().take(rows) {
                canvas.text(&line, cx, ty, WHITE);
                ty += 9;
            }
        }
    }
}

fn fit(s: &str, chars: usize) -> String {
    s.chars().take(chars).collect()
}

fn wrap(s: &str, chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split(' ') {
        if !line.is_empty() && line.len() + 1 + word.len() > chars {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn cost(kind: Kind) -> u32 {
    let s = spec(kind);
    s.salvage + s.pressure
}

fn plural(kind: Kind) -> String {
    // HEADQUARTERS and WORKS are their own plurals.
    if kind.name().ends_with('S') {
        kind.name().to_string()
    } else {
        format!("{}S", kind.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_content::TIDE_HOLD_TICKS;
    use bw_core::Faction;

    #[test]
    fn a_resumed_match_keeps_every_seats_history_from_the_first_tick() {
        let mut world = World::with_map(
            8,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Compact],
        )
        .unwrap();
        world.ai_enabled = false;
        for tick in 1..=600u64 {
            if tick == 300 {
                world.issue(2, bw_sim::Command::Surrender).unwrap();
            }
            world.step();
        }
        let dir = std::env::temp_dir().join(format!(
            "brinewake-history-{}-{}",
            std::process::id(),
            world.seed
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("resume.replay.json");
        world.export_replay(&path).unwrap();
        let mut player = bw_sim::ReplayPlayer::open(&path).unwrap();
        let history = History::play(&mut player).unwrap();
        assert_eq!(player.world().tick, 600);
        for seat in 0..3u8 {
            let (summary, dock) = history.for_seat(&world, seat).expect("same match");
            assert_eq!(
                summary.first_tick(),
                0,
                "the whole match, not from the resume"
            );
            assert_eq!(summary.samples.len(), 4);
            // Each seat's copy names the seats as its screen does: the
            // seat that gave up is seat 2 turned to this seat's view.
            let gone = (2 + 3 - seat) % 3;
            assert_eq!(summary.eliminated.len(), 1);
            assert_eq!(summary.eliminated[0].0, gone);
            assert!(
                dock.log
                    .entries
                    .iter()
                    .any(|entry| entry.kind == crate::dock_log::DockLogKind::Out)
            );
            assert!(!dock.log.history_partial);
            // The same as a screen that watched every tick of its view.
            let mut direct = MatchSummary::default();
            let mut replay = bw_sim::ReplayPlayer::open(&path).unwrap();
            while replay.step().unwrap() {
                direct.observe(&replay.world().relabeled_for(seat));
            }
            assert_eq!(direct, summary);
        }
        let _ = std::fs::remove_dir_all(&dir);
        let mut later = world.clone();
        later.tick += 1;
        assert!(history.for_seat(&later, 0).is_none(), "another tick");
    }

    /// `BW_RESUME_REPLAY=FILE cargo test --release -p bw_desktop
    /// resume_history_of_a_recording -- --ignored --nocapture`: what each
    /// seat's result screen would hold after `--resume FILE`.
    #[test]
    #[ignore]
    fn resume_history_of_a_recording() {
        let Ok(path) = std::env::var("BW_RESUME_REPLAY") else {
            return;
        };
        let mut player = bw_sim::ReplayPlayer::open(&path).unwrap();
        let history = History::play(&mut player).unwrap();
        for (seat, (summary, dock)) in history.seats.iter().enumerate() {
            let peak = summary
                .samples
                .iter()
                .map(|s| of(&s.army, 0usize))
                .max()
                .unwrap_or(0);
            println!(
                "seat {seat}: from {} to {}, {} samples, lost {}, peak army {peak}, {} log rows, {}",
                clock(summary.first_tick()),
                clock(history.tick),
                summary.samples.len(),
                summary.losses.iter().filter(|l| l.owner == 0).count(),
                dock.log.entries.len(),
                dock.log.history_label()
            );
        }
    }

    #[test]
    fn every_kind_that_dies_is_counted() {
        // bw_core has no list of kinds; this is every one a Death event
        // names, and the match in kind_named must stay exhaustive.
        let kinds = [
            Kind::Hook,
            Kind::Riveter,
            Kind::Bulwark,
            Kind::Sounder,
            Kind::Wick,
            Kind::Skipper,
            Kind::Reedguard,
            Kind::Loom,
            Kind::Headquarters,
            Kind::Works,
            Kind::Dropoff,
            Kind::Condenser,
            Kind::Tower,
            Kind::Tidewatch,
            Kind::Caulker,
            Kind::Caisson,
            Kind::Lampwright,
            Kind::Tender,
            Kind::Dredger,
            Kind::Drydock,
            Kind::Palisade,
            Kind::Barge,
            Kind::Lifter,
            Kind::Raker,
            Kind::Brander,
            Kind::Heliostat,
            Kind::Glinter,
            Kind::Stilt,
            Kind::Glazier,
            Kind::Salter,
            Kind::Pan,
        ];
        for kind in kinds {
            assert_eq!(kind_named(kind.name()), Some(kind), "{kind:?}");
        }
    }

    #[test]
    fn a_practice_match_is_sampled_every_five_seconds() {
        let mut world = World::new(7, Faction::Union);
        let mut summary = MatchSummary::default();
        for _ in 0..(SAMPLE_EVERY * 4) {
            world.step();
            summary.observe(&world);
        }
        assert_eq!(summary.first_tick(), 0);
        assert_eq!(summary.samples.len(), 4);
        assert_eq!(summary.samples[3].tick, SAMPLE_EVERY * 4);
        assert_eq!(summary.side_name(0), "BREAKWATER UNION");
        // Workers are gathering: salvage comes in, and none of it is army.
        assert!(summary.samples[3].income[0] > 0 || summary.samples[3].crew[0] > 0);
    }

    #[test]
    fn losses_hold_runs_and_the_turning_point_are_counted() {
        let mut summary = MatchSummary {
            factions: vec![Some(Faction::Union), Some(Faction::Assembly)],
            ..Default::default()
        };
        for i in 0..5 {
            summary.losses.push(Loss {
                tick: 9000 + i * 30,
                owner: 0,
                kind: Kind::Bulwark,
                cause: Some(Kind::Loom),
                pos: None,
                killer: None,
            });
        }
        summary.losses.push(Loss {
            tick: 12000,
            owner: 1,
            kind: Kind::Reedguard,
            cause: Some(Kind::Riveter),
            pos: None,
            killer: None,
        });
        assert_eq!(
            summary
                .losses
                .iter()
                .filter(|l| l.owner == 0 && l.kind == Kind::Bulwark)
                .count(),
            5
        );
        assert_eq!(summary.kills_by_kind(1), vec![(Kind::Loom, 5)]);
        assert_eq!(summary.kills_by_kind(0), vec![(Kind::Riveter, 1)]);
        let line = summary.turning_line().unwrap();
        assert!(
            line.starts_with("5:00 BREAKWATER UNION LOST 5 MACHINES"),
            "{line}"
        );
        assert!(line.ends_with("TO LOOMS."), "{line}");

        summary.holds.push(HoldRun {
            player: 1,
            start: 30000,
            end: Some(32700),
            peak: TIDE_HOLD_TICKS,
            won: true,
            breaks: 0,
            holding: true,
        });
        summary.outcome = Some((32700, Outcome::Victory(1)));
        summary.tide = true;
        assert_eq!(
            summary.decided_line(),
            "SILT ASSEMBLY HELD BOTH LANES FROM 16:40 TO 18:10."
        );
        assert_eq!(summary.best_hold(1), (TIDE_HOLD_TICKS, true));
        summary.holds[0].breaks = 7;
        assert_eq!(
            summary.decided_line(),
            "SILT ASSEMBLY FILLED ITS COUNT FROM 16:40 TO 18:10, BROKEN 7 TIMES BUT NEVER EMPTIED."
        );
        summary.holds[0].breaks = 0;
        assert_eq!(summary.best_hold(0), (0, false));
    }

    #[test]
    fn hold_runs_open_and_close_with_the_gauge() {
        let mut world = World::new(3, Faction::Union);
        let mut summary = MatchSummary::default();
        summary.observe(&world);
        for (tick, gauge) in [(1, 30), (2, 60), (3, 0), (4, 30)] {
            world.tick = tick;
            world.lane_hold = vec![0, gauge];
            summary.observe(&world);
        }
        assert_eq!(summary.holds.len(), 2);
        assert_eq!(summary.holds[0].start, 1);
        assert_eq!(summary.holds[0].end, Some(3));
        assert_eq!(summary.holds[0].peak, 60);
        assert_eq!(summary.holds[1].end, None);
    }

    fn confluence(seed: u64) -> World {
        World::with_map(
            seed,
            bw_sim::MapId::Confluence,
            &[Faction::Union, Faction::Assembly, Faction::Union],
        )
        .expect("confluence")
    }

    fn fell(world: &mut World, seat: u8) {
        for entity in &mut world.entities {
            if entity.owner == seat && entity.kind == Kind::Headquarters {
                entity.hp = 0;
            }
        }
        world.entities.retain(|entity| entity.hp > 0);
    }

    #[test]
    fn three_seats_are_sampled_per_seat_and_placed_as_they_go_out() {
        let mut world = confluence(3);
        world.ai_enabled = false;
        let mut summary = MatchSummary::default();
        for _ in 0..SAMPLE_EVERY {
            world.step();
            summary.observe(&world);
        }
        let sample = summary.latest().expect("a sample");
        assert_eq!(sample.army.len(), 3);
        assert_eq!(sample.income.len(), 3);
        assert_eq!(sample.crew.len(), 3);
        assert_eq!(sample.hold.len(), 3);
        assert_eq!(summary.sluice_share().len(), 3);
        assert!(summary.free_for_all());
        // Colours follow the players: this Confluence is all Union, so
        // the seats take the colours no Union body wears.
        assert_eq!(summary.seat_name(2, Some(0)), "PINK");
        assert_eq!(summary.seat_name(1, Some(0)), "JADE", "the Assembly's own");
        assert_eq!(summary.seat_name(0, None), "VIOLET");
        // Violet falls: third, and the match goes on.
        fell(&mut world, 2);
        world.step();
        summary.observe(&world);
        assert_eq!(world.outcome, None);
        assert_eq!(summary.place_of(2), Some(3));
        assert_eq!(summary.place_of(0), None);
        // Red gives up: the first seat wins and red is second.
        world.issue(1, bw_sim::Command::Surrender).unwrap();
        world.step();
        summary.observe(&world);
        assert_eq!(world.outcome, Some(Outcome::Victory(0)));
        assert_eq!(summary.placings(), vec![(0, 1), (1, 2), (2, 3)]);
        assert!(summary.surrendered);
        let out = clock(summary.out_at(1).unwrap());
        assert_eq!(
            summary.decided_line_for(Some(0)),
            format!("JADE SURRENDERED AT {out}.")
        );
    }

    #[test]
    fn the_last_headquarters_to_fall_and_a_tide_win_read_per_seat() {
        let mut world = confluence(5);
        world.ai_enabled = false;
        let mut summary = MatchSummary::default();
        world.step();
        summary.observe(&world);
        fell(&mut world, 0);
        world.step();
        summary.observe(&world);
        assert_eq!(summary.place_of(0), Some(3), "the person is out, third");
        fell(&mut world, 1);
        world.step();
        summary.observe(&world);
        assert_eq!(world.outcome, Some(Outcome::Victory(2)));
        assert!(!summary.surrendered);
        assert_eq!(summary.placings(), vec![(2, 1), (1, 2), (0, 3)]);
        let line = summary.decided_line_for(Some(0));
        assert!(line.starts_with("JADE WAS THE LAST TO FALL, AT "), "{line}");
        // A tide win names the winner's own crossings; the others share
        // second while they stand.
        let tide = MatchSummary {
            factions: vec![Some(Faction::Union); 3],
            holds: vec![HoldRun {
                player: 1,
                start: 30000,
                end: Some(32700),
                peak: TIDE_HOLD_TICKS,
                won: true,
                breaks: 0,
                holding: true,
            }],
            outcome: Some((32700, Outcome::Victory(1))),
            tide: true,
            ..Default::default()
        };
        assert_eq!(
            tide.decided_line_for(Some(0)),
            "JADE HELD ITS LANES FROM 16:40 TO 18:10."
        );
        assert_eq!(tide.placings(), vec![(1, 1), (0, 2), (2, 2)]);
    }

    #[test]
    fn three_seat_kills_count_only_for_the_seat_that_made_them() {
        let mut summary = MatchSummary {
            factions: vec![Some(Faction::Union); 3],
            ..Default::default()
        };
        for (owner, killer) in [(1, Some(2)), (1, Some(2)), (2, Some(0)), (0, None)] {
            summary.losses.push(Loss {
                tick: 100,
                owner,
                kind: Kind::Hook,
                cause: Some(Kind::Riveter),
                pos: None,
                killer,
            });
        }
        assert_eq!(summary.kills_by_kind(2), vec![(Kind::Riveter, 2)]);
        assert_eq!(summary.kills_by_kind(0), vec![(Kind::Riveter, 1)]);
        assert!(summary.kills_by_kind(1).is_empty());
    }

    #[test]
    fn a_kill_names_the_killer_seat_even_when_it_fell_the_same_tick() {
        let mut world = confluence(6);
        let red = world
            .entities
            .iter()
            .find(|e| e.owner == 1 && !e.kind.is_building())
            .map(|e| e.id)
            .expect("a red machine");
        let death = |tick: u64, other: u32| bw_sim::Event {
            tick,
            kind: EventKind::Death,
            player: Some(2),
            entity: Some(9999),
            other: Some(other),
            from: None,
            to: None,
            amount: 0,
            text: "HOOK".into(),
            cause: Some(Kind::Hook),
        };
        assert_eq!(killer_seat(&world, &death(0, red)), Some(1));
        // A killer gone from the world is found by this tick's blow.
        let blow = bw_sim::Event {
            kind: EventKind::Damage,
            player: Some(0),
            ..death(0, 424_242)
        };
        world.events.push(blow);
        assert_eq!(killer_seat(&world, &death(0, 424_242)), Some(0));
        assert_eq!(killer_seat(&world, &death(0, 1_000_000)), None);
    }
}
