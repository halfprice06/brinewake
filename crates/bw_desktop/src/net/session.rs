//! The lockstep match between peers.
//!
//! Every seat runs the same deterministic simulation. A seat's orders from
//! tick `t` are stamped for `t + delay` and sent to everyone; a tick is
//! stepped only once every playing seat's batch for it is in hand, so the
//! worlds never part. Every thirty ticks each seat sends its state hash and
//! any difference ends the match as a desync rather than playing on.
//!
//! This is the trusted mode from the plan: each peer holds the whole world,
//! which suits friends and agents, not public play.
//!
//! **The hub.** The host is a hub: a guest sends only to the host, and the
//! host passes each guest's batches and hashes on to every other guest. So
//! only the host must be reachable, and nothing here counts seats: the
//! plan's seats decide who plays and who watches.
//!
//! **Input delay** is each seat's own: its batches must reach the others in
//! time, so it follows the longest path its orders travel, measured every
//! second. Raising it fills the skipped ticks with empty batches; lowering
//! it waits until the stamp catches up. No seat needs to agree to either.
//!
//! **Pace.** A seat that has run ahead of the others (it started a little
//! earlier, or its clock is faster) holds a tick now and then. Otherwise it
//! would spend the delay's margin waiting and stall on every hiccup.
//!
//! **Leaving.** A goodbye drops a seat at once; silence drops it after a
//! minute, during which the others wait and a restarted game can rejoin
//! with its token. The host decides a drop and names the first tick without
//! the seat's batches, where that seat surrenders; every seat applies it at
//! the same tick. A guest whose host has gone plays on alone only when the
//! host was its one opponent, and then it has won.
//!
//! **A seat kept open.** While a seat is silent the host may keep it open
//! instead (`keep_seat_open`, KEEP SEAT OPEN on its screen) for up to
//! `KEEP_OPEN`. The others play on without it: the host stands in for the
//! seat and sends its batches, empty but for one order at the start that
//! has its machines hold where they stand. Those batches travel like any
//! other, so every seat applies them on the same ticks and nothing else
//! needs agreeing. The seat has not surrendered. When its player joins
//! again (with its token, or by its lobby name and the host's code) the
//! host stops standing in and sends it the world and every batch still to
//! come (`Resume`); its own batches carry on from the last one the host
//! sent for it. When the time runs out the seat is dropped as above.

use super::link::{Endpoint, LinkEvent, PeerId};
use super::protocol::{MatchPlan, PROTOCOL, Wire};
use bw_core::Faction;
use bw_sim::{Command, World};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub const DEFAULT_DELAY: u64 = 3;
/// Ticks between state hash exchanges.
pub const HASH_EVERY: u64 = 30;
const TICK_MS: f64 = 1000.0 / 30.0;
/// A seat silent this long is dropped from the match. Silence is measured
/// on the connection, which the link's own thread keeps alive with
/// keepalives and acknowledgements: a seat whose game is busy (a slow tick)
/// is still heard, and is waited for rather than dropped. Only a seat whose
/// connection has gone quiet counts towards this.
pub const DROP_AFTER: Duration = Duration::from_secs(60);
/// How long the host may keep a silent seat open for its player to come
/// back, while the others play on. After this the seat is dropped.
pub const KEEP_OPEN: Duration = Duration::from_secs(300);
/// Silence shorter than this is a hiccup, not worth a word.
const QUIET_SHOWN: Duration = Duration::from_secs(2);
pub const MIN_DELAY: u64 = 2;
pub const MAX_DELAY: u64 = 15;
/// Margin over the one-way time for frame timing and scheduling.
const DELAY_MARGIN_MS: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepOutcome {
    /// One tick advanced on the canonical world.
    Stepped,
    /// A batch for this tick has not arrived yet, or this seat is holding
    /// back to keep pace.
    Stalled,
    /// The session is over: match decided, host gone, or desynchronised.
    Ended,
}

/// Why a match stopped before it was decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ending {
    /// The seats' worlds differ from `tick`; `seats` disagree with this one.
    Desync { tick: u64, seats: Vec<u8> },
    /// The host left, and the match cannot go on without it.
    HostLeft,
    /// The host gave up on this seat.
    Dropped,
}

enum Hub {
    Host {
        /// Each seat's connection; `None` for the host's own seat and for a
        /// guest that has gone quiet or dropped.
        peers: Vec<Option<PeerId>>,
        tokens: Vec<u64>,
        gone_since: Vec<Option<Instant>>,
        /// Connections that have not said who they are yet.
        strangers: Vec<(PeerId, Instant)>,
        paths_sent: Instant,
    },
    Guest {
        host: PeerId,
        gone_since: Option<Instant>,
    },
}

/// What the field shows about the network.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetView {
    /// Round trip to the host (a guest) or the slowest guest (the host).
    pub rtt_ms: Option<u32>,
    pub delay: u64,
    /// The seat this one is waiting for, its name, and how long it has been
    /// silent and how long before it is dropped. Only a seat whose
    /// connection is silent: the drop clock is running.
    pub waiting: Option<(u8, String, u64, u64)>,
    /// A seat still connected (its keepalives arrive) whose orders are late,
    /// its name, and for how many seconds: a busy or slow game. No drop
    /// clock runs for it.
    pub slow: Option<(u8, String, u64)>,
    /// A seat kept open while its player is away, its name, and seconds
    /// before it is dropped: the one with the least time left.
    pub away: Option<(u8, String, u64)>,
    /// This seat (the host) may keep the waited-for seat open.
    pub can_keep_open: bool,
    pub watching: bool,
}

pub struct Session {
    /// This peer's seat in the plan.
    pub local_seat: u8,
    /// This peer's player in the canonical world (0 for a watcher, who
    /// sees the first player's side with the fog lifted).
    pub local: u8,
    pub seed: u64,
    pub plan: MatchPlan,
    /// This seat's current input delay in ticks.
    pub delay: u64,
    fixed_delay: bool,
    players: Vec<Option<u8>>,
    canonical: World,
    scheduled: BTreeMap<u64, Vec<(u8, Command)>>,
    /// Per seat, the last tick whose batch is in hand (sent, for this seat).
    upto: Vec<u64>,
    /// Per seat, the first tick it no longer plays.
    dropped: Vec<Option<u64>>,
    /// Per seat, the tick it last reported standing on, and when.
    heard_tick: Vec<Option<(u64, Instant)>>,
    outbox: Vec<Command>,
    hashes: BTreeMap<u64, Vec<Option<String>>>,
    pub desync: Option<u64>,
    ending: Option<Ending>,
    /// True once the match cannot continue for any reason but a desync.
    pub disconnected: bool,
    link: Endpoint,
    hub: Hub,
    starts_at: Instant,
    rejection: Option<String>,
    lower_votes: u32,
    /// Each seat's round trip to the host.
    path_rtts: Vec<Option<u32>>,
    /// Each seat's connection silence as the host last measured it, in ms:
    /// how a guest knows whether another guest is gone or only slow.
    path_quiet: Vec<Option<u32>>,
    /// Silence after which a seat is dropped: `DROP_AFTER`, shorter in tests.
    drop_after: Duration,
    /// Per seat, while it is kept open for its player to come back, when it
    /// will be dropped. The host decides; guests hear it with `Paths`.
    away_until: Vec<Option<Instant>>,
    /// Host: seats kept open whose machines have not yet been told to hold.
    hold_due: Vec<bool>,
    /// How long a seat may be kept open: `KEEP_OPEN`, shorter in tests.
    keep_open_for: Duration,
    notices: Vec<String>,
    closed: bool,
    _port_mapping: Option<super::upnp::PortMapping>,
}

fn rules_digest() -> String {
    bw_content::rules_digest()
}

/// Refuse a peer built from different source. The digest covers the content
/// constants; this covers everything else a tick depends on.
pub fn check_build(peer: &str) -> Result<(), String> {
    let own = bw_content::build_fingerprint();
    if peer == own || own == "unknown" || peer == "unknown" {
        return Ok(());
    }
    Err(format!(
        "The other player runs a different build ({peer}; this one is {own}). Both must run the same one."
    ))
}

/// Refuse a guest the host cannot play with, saying why.
pub fn check_hello(protocol: u32, digest: &str, build: &str) -> Result<(), String> {
    if protocol != PROTOCOL {
        return Err(format!(
            "The other player's game speaks version {protocol}; this one speaks {PROTOCOL}. Both must update."
        ));
    }
    if digest != rules_digest() {
        return Err(
            "The other player's game has different rules. Both must run the same build.".into(),
        );
    }
    check_build(build)
}

/// Start options, collected so a lobby can hand them over in one piece.
pub(super) struct Start {
    pub link: Endpoint,
    pub plan: MatchPlan,
    pub local_seat: u8,
    pub delay: u64,
    pub fixed_delay: bool,
    pub starts_at: Instant,
    /// The router's forwarding, kept open for the match so a guest can
    /// rejoin.
    pub port_mapping: Option<super::upnp::PortMapping>,
}

/// A match to carry on from, set from the command line (`--resume`): every
/// seat of a match on one machine replays the same recording to its end and
/// the session starts there instead of at tick zero.
static RESUME: std::sync::OnceLock<World> = std::sync::OnceLock::new();
/// What every seat's screen would have gathered over the resumed match's
/// ticks so far: its summary and dock log, rebuilt while the recording was
/// played through.
static RESUME_HISTORY: std::sync::OnceLock<crate::match_summary::History> =
    std::sync::OnceLock::new();

/// Carry the next match on from `world`, a recording played to its end, with
/// the history gathered on the way there.
pub fn resume_from(world: World, history: crate::match_summary::History) -> Result<(), String> {
    RESUME
        .set(world)
        .map_err(|_| "a match to resume is already set".to_string())?;
    let _ = RESUME_HISTORY.set(history);
    Ok(())
}

/// The summary and dock log `seat` (a canonical seat) would hold at
/// `canonical`, when that is the resumed match at the tick it resumed from.
pub fn resume_history(
    canonical: &World,
    seat: u8,
) -> Option<(
    crate::match_summary::MatchSummary,
    crate::dock_log::DockState,
)> {
    RESUME_HISTORY.get()?.for_seat(canonical, seat)
}

/// The world a new session starts from: the plan's, or the recording set
/// with `resume_from` when it is the same match.
fn starting_world(plan: &MatchPlan) -> Result<World, String> {
    let fresh = plan.world()?;
    let Some(resumed) = RESUME.get() else {
        return Ok(fresh);
    };
    let factions = |w: &World| w.players.iter().map(|p| p.faction).collect::<Vec<_>>();
    if resumed.map.id != fresh.map.id || factions(resumed) != factions(&fresh) {
        return Err("The recording to resume is not this match's map and sides.".into());
    }
    let mut world = resumed.clone();
    world.ai_enabled = false;
    Ok(world)
}

impl Session {
    fn build(start: Start, hub: Hub, world: World) -> Session {
        let seats = start.plan.seats.len();
        let players = start.plan.players();
        let local = players[start.local_seat as usize].unwrap_or(0);
        let delay = start.delay.clamp(1, MAX_DELAY);
        let first_tick = world.tick;
        Session {
            local_seat: start.local_seat,
            local,
            seed: start.plan.seed,
            plan: start.plan,
            delay,
            fixed_delay: start.fixed_delay,
            players,
            canonical: world,
            scheduled: BTreeMap::new(),
            // Ticks before the first delay carry no orders from anyone.
            upto: vec![first_tick + delay - 1; seats],
            dropped: vec![None; seats],
            heard_tick: vec![None; seats],
            outbox: Vec::new(),
            hashes: BTreeMap::new(),
            desync: None,
            ending: None,
            disconnected: false,
            link: start.link,
            hub,
            starts_at: start.starts_at,
            rejection: None,
            lower_votes: 0,
            path_rtts: vec![None; seats],
            path_quiet: vec![None; seats],
            drop_after: DROP_AFTER,
            away_until: vec![None; seats],
            hold_due: vec![false; seats],
            keep_open_for: KEEP_OPEN,
            notices: Vec::new(),
            closed: false,
            _port_mapping: start.port_mapping,
        }
    }

    pub(super) fn host(
        start: Start,
        peers: Vec<Option<PeerId>>,
        tokens: Vec<u64>,
    ) -> Result<Session, String> {
        let world = starting_world(&start.plan)?;
        let seats = start.plan.seats.len();
        let hub = Hub::Host {
            peers,
            tokens,
            gone_since: vec![None; seats],
            strangers: Vec::new(),
            paths_sent: Instant::now(),
        };
        Ok(Session::build(start, hub, world))
    }

    pub(super) fn guest(start: Start, host: PeerId) -> Result<Session, String> {
        let world = starting_world(&start.plan)?;
        Ok(Session::build(
            start,
            Hub::Guest {
                host,
                gone_since: None,
            },
            world,
        ))
    }

    /// A rejoining guest picks up the match where the host has it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resume(
        link: Endpoint,
        host: PeerId,
        seat: u8,
        plan: MatchPlan,
        world: World,
        batches: Vec<(u64, u8, Vec<Command>)>,
        upto: Vec<u64>,
        dropped: Vec<Option<u64>>,
        delay: u64,
    ) -> Result<Session, String> {
        let seats = plan.seats.len();
        if upto.len() != seats || dropped.len() != seats || seat as usize >= seats {
            return Err("The host sent a match this game cannot read.".into());
        }
        let start = Start {
            link,
            plan,
            local_seat: seat,
            delay,
            fixed_delay: false,
            starts_at: Instant::now(),
            port_mapping: None,
        };
        let mut session = Session::build(
            start,
            Hub::Guest {
                host,
                gone_since: None,
            },
            world,
        );
        for (tick, seat, commands) in batches {
            session
                .scheduled
                .entry(tick)
                .or_default()
                .extend(commands.into_iter().map(|c| (seat, c)));
        }
        session.upto = upto;
        session.dropped = dropped;
        Ok(session)
    }

    /// When the first tick is played, as every seat agreed.
    pub fn starts_at(&self) -> Instant {
        self.starts_at
    }

    pub fn is_host(&self) -> bool {
        matches!(self.hub, Hub::Host { .. })
    }

    pub fn watching(&self) -> bool {
        self.players[self.local_seat as usize].is_none()
    }

    pub fn local_faction(&self) -> Faction {
        self.canonical.players[self.local as usize].faction
    }

    pub fn canonical(&self) -> &World {
        &self.canonical
    }

    pub fn tick(&self) -> u64 {
        self.canonical.tick
    }

    pub fn ending(&self) -> Option<&Ending> {
        self.ending.as_ref()
    }

    /// The name of a seat's player.
    pub fn seat_name(&self, seat: u8) -> &str {
        self.plan
            .seats
            .get(seat as usize)
            .map_or("PLAYER", |s| s.name.as_str())
    }

    /// The world as this seat presents it: player 0 is always the local
    /// side, and a watcher sees everything.
    pub fn view(&self) -> World {
        let mut view = self.canonical.relabeled_for(self.local);
        // A watcher sees everything, and so does a seat knocked out of a
        // match of three that goes on without it.
        if self.watching() || view.is_eliminated(0) {
            view.revealed = true;
        }
        view
    }

    /// Validate now for feedback; the command executes `delay` ticks later on
    /// every peer, where it is validated again.
    pub fn issue(&mut self, command: Command) -> Result<(), String> {
        if self.watching() {
            return Err("You are watching this match.".to_string());
        }
        if self.desync.is_some() {
            return Err("The match has desynchronised.".to_string());
        }
        if self.ending.is_some() {
            return Err("The match is over.".to_string());
        }
        self.canonical.validate(self.local, &command)?;
        self.outbox.push(command);
        Ok(())
    }

    /// This seat's orders that were accepted but have not reached the world
    /// yet: the outbox, and its batches scheduled for coming ticks. A batch
    /// of orders reserves against these, since the world has not spent
    /// anything for them.
    pub fn pending_local(&self) -> Vec<Command> {
        let now = self.canonical.tick;
        self.scheduled
            .range(now..)
            .flat_map(|(_, batch)| batch.iter())
            .filter(|(seat, _)| *seat == self.local_seat)
            .map(|(_, command)| command.clone())
            .chain(self.outbox.iter().cloned())
            .collect()
    }

    /// Send a message from this seat to everyone else.
    fn broadcast(&mut self, message: &Wire) {
        let bytes = message.encode();
        match &self.hub {
            Hub::Host { peers, .. } => {
                for peer in peers.iter().flatten() {
                    let _ = self.link.send(*peer, &bytes);
                }
            }
            Hub::Guest { host, .. } => {
                let _ = self.link.send(*host, &bytes);
            }
        }
    }

    /// The host passes a guest's message on to every other guest.
    fn relay(&mut self, from_seat: u8, bytes: &[u8]) {
        if let Hub::Host { peers, .. } = &self.hub {
            for (seat, peer) in peers.iter().enumerate() {
                if seat != from_seat as usize
                    && let Some(peer) = peer
                {
                    let _ = self.link.send(*peer, bytes);
                }
            }
        }
    }

    fn seat_of(&self, peer: PeerId) -> Option<u8> {
        match &self.hub {
            Hub::Host { peers, .. } => peers.iter().position(|p| *p == Some(peer)).map(|s| s as u8),
            Hub::Guest { .. } => None,
        }
    }

    fn is_host_peer(&self, peer: PeerId) -> bool {
        matches!(&self.hub, Hub::Guest { host, .. } if *host == peer)
    }

    fn accept_batch(&mut self, seat: u8, tick: u64, now: u64, commands: Vec<Command>) -> bool {
        let s = seat as usize;
        if s >= self.upto.len()
            || s == self.local_seat as usize
            || self.players[s].is_none()
            || self.dropped[s].is_some_and(|from| tick >= from)
            || tick <= self.upto[s]
        {
            return false;
        }
        self.scheduled
            .entry(tick)
            .or_default()
            .extend(commands.into_iter().map(|c| (seat, c)));
        self.upto[s] = tick;
        self.heard_tick[s] = Some((now, Instant::now()));
        true
    }

    fn record_hash(&mut self, seat: u8, tick: u64, hash: String) {
        let seats = self.plan.seats.len();
        if (seat as usize) < seats && tick + HASH_EVERY * 20 >= self.canonical.tick {
            self.hashes.entry(tick).or_insert_with(|| vec![None; seats])[seat as usize] =
                Some(hash);
        }
    }

    /// Drop a seat: from `from_tick` on it has no batches and it has
    /// surrendered.
    fn apply_drop(&mut self, seat: u8, from_tick: u64) {
        let s = seat as usize;
        if s >= self.dropped.len() || self.dropped[s].is_some() {
            return;
        }
        self.dropped[s] = Some(from_tick);
        self.away_until[s] = None;
        self.hold_due[s] = false;
        if seat == self.local_seat {
            self.ending = Some(Ending::Dropped);
            self.disconnected = true;
        } else {
            let name = self.seat_name(seat).to_string();
            self.notices.push(format!("{name} LEFT"));
        }
    }

    /// The host gives up on a guest seat and tells everyone.
    fn drop_seat(&mut self, seat: u8) {
        let s = seat as usize;
        if self.dropped[s].is_some() || !self.is_host() {
            return;
        }
        // No seat can have stepped past the last batch the host holds for
        // this one: the host passes on everything it accepts.
        let from_tick = (self.upto[s] + 1).max(self.canonical.tick);
        if let Hub::Host { peers, .. } = &mut self.hub
            && let Some(peer) = peers[s].take()
        {
            self.link.disconnect(peer);
        }
        self.apply_drop(seat, from_tick);
        self.broadcast(&Wire::Dropped { seat, from_tick });
    }

    /// A guest has lost its host.
    fn host_left(&mut self) {
        if self.ending.is_some() {
            return;
        }
        let others: Vec<u8> = (0..self.plan.seats.len() as u8)
            .filter(|s| *s != self.local_seat && self.players[*s as usize].is_some())
            .filter(|s| self.dropped[*s as usize].is_none())
            .collect();
        // Play on alone only when the host was this seat's one opponent:
        // then the host has surrendered, and this seat has won.
        if !self.watching() && others == [0] && self.players[0].is_some() {
            let from = self.upto[0] + 1;
            self.apply_drop(0, from);
            self.disconnected = true;
        } else {
            self.ending = Some(Ending::HostLeft);
            self.disconnected = true;
        }
    }

    /// Everything the network brought since the last call.
    fn pump(&mut self) {
        let now = Instant::now();
        while let Some(event) = self.link.poll() {
            match event {
                LinkEvent::Message { peer, bytes } => {
                    let Ok(message) = Wire::decode(&bytes) else {
                        continue;
                    };
                    self.handle(peer, message, &bytes);
                }
                LinkEvent::Connected { peer, .. } => {
                    if let Hub::Host { strangers, .. } = &mut self.hub {
                        strangers.push((peer, now));
                    }
                }
                LinkEvent::Closed { peer } => match &mut self.hub {
                    Hub::Host {
                        peers, gone_since, ..
                    } => {
                        if let Some(seat) = peers.iter().position(|p| *p == Some(peer)) {
                            peers[seat] = None;
                            gone_since[seat].get_or_insert(now);
                        }
                    }
                    Hub::Guest {
                        host, gone_since, ..
                    } if *host == peer => {
                        gone_since.get_or_insert(now);
                    }
                    Hub::Guest { .. } => {}
                },
                LinkEvent::ConnectFailed { .. } | LinkEvent::Mapped(_) => {}
            }
        }
        self.check_liveness(now);
    }

    fn handle(&mut self, peer: PeerId, message: Wire, bytes: &[u8]) {
        match message {
            Wire::Batch {
                seat,
                tick,
                now,
                commands,
            } => {
                if self.is_host() {
                    if self.seat_of(peer) == Some(seat)
                        && self.accept_batch(seat, tick, now, commands)
                    {
                        self.relay(seat, bytes);
                    }
                } else if self.is_host_peer(peer) {
                    self.accept_batch(seat, tick, now, commands);
                }
            }
            Wire::Hash { seat, tick, hash } => {
                if self.is_host() {
                    if self.seat_of(peer) == Some(seat) {
                        self.record_hash(seat, tick, hash);
                        self.relay(seat, bytes);
                    }
                } else if self.is_host_peer(peer) && seat != self.local_seat {
                    self.record_hash(seat, tick, hash);
                }
            }
            Wire::Dropped { seat, from_tick } if self.is_host_peer(peer) => {
                self.apply_drop(seat, from_tick);
            }
            Wire::Paths {
                rtt_ms,
                quiet_ms,
                away_ms,
            } if self.is_host_peer(peer) => {
                if rtt_ms.len() == self.path_rtts.len() {
                    self.path_rtts = rtt_ms;
                }
                if quiet_ms.len() == self.path_quiet.len() {
                    self.path_quiet = quiet_ms;
                }
                if away_ms.len() == self.away_until.len() {
                    let now = Instant::now();
                    self.away_until = away_ms
                        .iter()
                        .map(|ms| ms.map(|ms| now + Duration::from_millis(ms.into())))
                        .collect();
                }
            }
            Wire::Bye { .. } if self.is_host_peer(peer) => self.host_left(),
            Wire::Bye { seat } => {
                if self.seat_of(peer) == Some(seat) {
                    self.drop_seat(seat);
                }
            }
            Wire::Hello {
                protocol,
                rules_digest,
                build,
                rejoin,
                name,
            } if self.is_host() => {
                self.hello_in_match(peer, protocol, &rules_digest, &build, rejoin, &name)
            }
            _ => {}
        }
    }

    /// The seat a guest coming back takes: the one its token names, or else
    /// a seat that has lost its player and goes by the name it gives (its
    /// lobby name; "ANNE" also finds "ANNE 2", the name the lobby gave a
    /// second ANNE, when only one such seat is open). The host's own seat and
    /// a dropped one are never taken.
    fn returning_seat(&self, rejoin: Option<u64>, name: &str) -> Option<usize> {
        let Hub::Host { tokens, peers, .. } = &self.hub else {
            return None;
        };
        let takeable = |s: usize| {
            s != self.local_seat as usize && self.dropped[s].is_none() && self.players[s].is_some()
        };
        if let Some(seat) = rejoin
            .and_then(|token| tokens.iter().position(|t| *t == token))
            .filter(|s| takeable(*s))
        {
            return Some(seat);
        }
        // By name only a seat whose player is gone: kept open, or silent.
        let open = |s: usize| {
            takeable(s)
                && (self.away_until[s].is_some()
                    || peers[s].is_none()
                    || self.quiet_of(s as u8).is_some_and(|q| q >= QUIET_SHOWN))
        };
        let name = super::protocol::clean_name(name);
        let seats = 0..self.plan.seats.len();
        if let Some(seat) = seats
            .clone()
            .find(|s| open(*s) && self.plan.seats[*s].name == name)
        {
            return Some(seat);
        }
        // The lobby names a second ANNE "ANNE 2": the first eight letters and
        // a number.
        let short = &name[..name.len().min(8)];
        let alike: Vec<usize> = seats
            .filter(|s| open(*s))
            .filter(|s| {
                self.plan.seats[*s]
                    .name
                    .rsplit_once(' ')
                    .is_some_and(|(prefix, n)| {
                        prefix == short && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
                    })
            })
            .collect();
        (alike.len() == 1).then(|| alike[0])
    }

    /// A connection during the match: a guest coming back, or a stranger.
    fn hello_in_match(
        &mut self,
        peer: PeerId,
        protocol: u32,
        digest: &str,
        build: &str,
        rejoin: Option<u64>,
        name: &str,
    ) {
        let refuse = |link: &Endpoint, reason: &str| {
            let _ = link.send(
                peer,
                &Wire::Refuse {
                    reason: reason.to_string(),
                }
                .encode(),
            );
            link.flush(Duration::from_millis(200));
            link.disconnect(peer);
        };
        if let Err(reason) = check_hello(protocol, digest, build) {
            refuse(&self.link, &reason);
            return;
        }
        let seat = self.returning_seat(rejoin, name);
        let Hub::Host {
            peers,
            tokens,
            gone_since,
            strangers,
            paths_sent,
            ..
        } = &mut self.hub
        else {
            return;
        };
        strangers.retain(|(p, _)| *p != peer);
        let Some(seat) = seat else {
            let given_up = rejoin
                .and_then(|token| tokens.iter().position(|t| *t == token))
                .is_some_and(|s| self.dropped[s].is_some());
            refuse(
                &self.link,
                if given_up {
                    "Your seat was given up: the match went on without you."
                } else {
                    "A match is already under way."
                },
            );
            return;
        };
        if let Some(old) = peers[seat].replace(peer)
            && old != peer
        {
            self.link.disconnect(old);
        }
        gone_since[seat] = None;
        // Everyone hears at once that the seat is back, not in a second.
        *paths_sent = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or(*paths_sent);
        let token = tokens[seat];
        // The host no longer stands in: the seat's own batches follow on
        // from the last one the host sent for it.
        self.away_until[seat] = None;
        self.hold_due[seat] = false;
        let tick = self.canonical.tick;
        let mut batches = Vec::new();
        for (at, batch) in self.scheduled.range(tick..) {
            for (s, command) in batch {
                match batches.last_mut() {
                    Some((t, last_seat, commands)) if *t == *at && *last_seat == *s => {
                        let commands: &mut Vec<Command> = commands;
                        commands.push(command.clone());
                    }
                    _ => batches.push((*at, *s, vec![command.clone()])),
                }
            }
        }
        let resume = Wire::Resume {
            seat: seat as u8,
            plan: self.plan.clone(),
            world: Box::new(self.canonical.clone()),
            batches,
            upto: self.upto.clone(),
            dropped: self.dropped.clone(),
            delay: DEFAULT_DELAY.max(self.delay),
        };
        let _ = self.link.send(
            peer,
            &Wire::Welcome {
                seat: seat as u8,
                token,
            }
            .encode(),
        );
        let _ = self.link.send(peer, &resume.encode());
        let name = self.seat_name(seat as u8).to_string();
        self.notices.push(format!("{name} IS BACK"));
    }

    fn quiet_of(&self, seat: u8) -> Option<Duration> {
        let s = seat as usize;
        match &self.hub {
            Hub::Host {
                peers, gone_since, ..
            } => {
                if s == self.local_seat as usize {
                    return None;
                }
                match peers[s] {
                    Some(peer) => self.link.stats(peer).map(|st| st.quiet),
                    None => gone_since[s].map(|at| at.elapsed()),
                }
            }
            Hub::Guest { host, gone_since } => {
                let to_host = gone_since
                    .map(|at| at.elapsed())
                    .or_else(|| self.link.stats(*host).map(|st| st.quiet));
                if s == 0 {
                    to_host
                } else {
                    // Another guest is heard through the host, which
                    // measures its connection and says so every second. Its
                    // late orders are not silence: a busy seat still sends
                    // keepalives, and only the host decides a drop.
                    let reported = self.path_quiet[s].map(|ms| Duration::from_millis(ms.into()));
                    match (to_host, reported) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (a, b) => a.or(b),
                    }
                }
            }
        }
    }

    fn check_liveness(&mut self, now: Instant) {
        if self.ending.is_some() || self.closed {
            return;
        }
        match &self.hub {
            Hub::Host { .. } => {
                for seat in 0..self.plan.seats.len() as u8 {
                    if seat == self.local_seat || self.dropped[seat as usize].is_some() {
                        continue;
                    }
                    // A seat kept open is dropped when its time is up, not
                    // for its silence.
                    let out = match self.away_until[seat as usize] {
                        Some(until) => now >= until,
                        None => self.quiet_of(seat).is_some_and(|q| q >= self.drop_after),
                    };
                    if out {
                        self.drop_seat(seat);
                    }
                }
                self.send_paths(now);
            }
            Hub::Guest { .. } => {
                if self.quiet_of(0).is_some_and(|q| q >= self.drop_after) {
                    self.host_left();
                }
            }
        }
        if let Hub::Host { strangers, .. } = &mut self.hub {
            // A connection that never says who it is goes.
            let stale: Vec<PeerId> = strangers
                .iter()
                .filter(|(_, at)| now.duration_since(*at) > Duration::from_secs(10))
                .map(|(p, _)| *p)
                .collect();
            strangers.retain(|(_, at)| now.duration_since(*at) <= Duration::from_secs(10));
            for peer in stale {
                self.link.disconnect(peer);
            }
        }
    }

    fn send_paths(&mut self, now: Instant) {
        let Hub::Host {
            peers,
            paths_sent,
            gone_since,
            ..
        } = &mut self.hub
        else {
            return;
        };
        if now.duration_since(*paths_sent) < Duration::from_secs(1) {
            return;
        }
        *paths_sent = now;
        let millis = |d: Duration| u32::try_from(d.as_millis()).unwrap_or(u32::MAX);
        let quiet: Vec<Option<u32>> = peers
            .iter()
            .enumerate()
            .map(|(seat, peer)| {
                if seat == self.local_seat as usize {
                    Some(0)
                } else {
                    match peer {
                        Some(p) => self.link.stats(*p).map(|s| millis(s.quiet)),
                        None => gone_since[seat].map(|at| millis(now.duration_since(at))),
                    }
                }
            })
            .collect();
        let rtts: Vec<Option<u32>> = peers
            .iter()
            .enumerate()
            .map(|(seat, peer)| {
                if seat == self.local_seat as usize {
                    Some(0)
                } else {
                    peer.and_then(|p| self.link.stats(p)).and_then(|s| s.rtt_ms)
                }
            })
            .collect();
        let away: Vec<Option<u32>> = self
            .away_until
            .iter()
            .map(|until| until.map(|at| millis(at.saturating_duration_since(now))))
            .collect();
        self.path_rtts = rtts.clone();
        self.path_quiet = quiet.clone();
        self.broadcast(&Wire::Paths {
            rtt_ms: rtts,
            quiet_ms: quiet,
            away_ms: away,
        });
    }

    /// Round trip and jitter of the path from this seat to `seat`.
    fn path_to(&self, seat: u8) -> (f64, f64) {
        let stats = |peer: PeerId| {
            self.link
                .stats(peer)
                .map(|s| (f64::from(s.rtt_ms.unwrap_or(100)), f64::from(s.jitter_ms)))
                .unwrap_or((100.0, 0.0))
        };
        match &self.hub {
            Hub::Host { peers, .. } => peers[seat as usize].map_or((0.0, 0.0), stats),
            Hub::Guest { host, .. } => {
                let (rtt, jitter) = stats(*host);
                let beyond = if seat == 0 {
                    0.0
                } else {
                    f64::from(self.path_rtts[seat as usize].unwrap_or(0))
                };
                (rtt + beyond, jitter)
            }
        }
    }

    fn other_players(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.plan.seats.len() as u8).filter(|s| {
            *s != self.local_seat
                && self.players[*s as usize].is_some()
                && self.dropped[*s as usize].is_none()
        })
    }

    /// True when this seat has run clearly ahead of another player.
    fn ahead_of_others(&self) -> bool {
        let tick = self.canonical.tick as f64;
        self.other_players().any(|seat| {
            let Some((their, at)) = self.heard_tick[seat as usize] else {
                return false;
            };
            let age = at.elapsed();
            if age > Duration::from_secs(1) {
                // Silent: the stall will say so; pacing has nothing to go on.
                return false;
            }
            let (rtt, _) = self.path_to(seat);
            let estimate = their as f64 + (age.as_secs_f64() * 1000.0 + rtt / 2.0) / TICK_MS;
            tick > estimate + 1.5
        })
    }

    /// Size the input delay to the longest path this seat's orders travel.
    fn review_delay(&mut self) {
        if self.fixed_delay || self.watching() {
            return;
        }
        let one_way = self
            .other_players()
            .map(|seat| {
                let (rtt, jitter) = self.path_to(seat);
                rtt / 2.0 + 2.0 * jitter
            })
            .fold(0.0, f64::max);
        let target =
            (((one_way + DELAY_MARGIN_MS) / TICK_MS).ceil() as u64).clamp(MIN_DELAY, MAX_DELAY);
        if target > self.delay {
            self.delay = target;
            self.lower_votes = 0;
        } else if target < self.delay {
            // Come down slowly: a path that was slow a moment ago may be
            // again.
            self.lower_votes += 1;
            if self.lower_votes >= 3 {
                self.delay -= 1;
                self.lower_votes = 0;
            }
        } else {
            self.lower_votes = 0;
        }
    }

    /// Send this seat's batches up to `tick + delay`.
    fn send_own(&mut self, tick: u64) {
        let seat = self.local_seat;
        if self.watching() || self.dropped[seat as usize].is_some() {
            return;
        }
        let due = tick + self.delay;
        let from = self.upto[seat as usize] + 1;
        if from > due {
            return;
        }
        for at in from..=due {
            let commands = if at == due {
                std::mem::take(&mut self.outbox)
            } else {
                Vec::new()
            };
            self.scheduled
                .entry(at)
                .or_default()
                .extend(commands.iter().cloned().map(|c| (seat, c)));
            self.broadcast(&Wire::Batch {
                seat,
                tick: at,
                now: tick,
                commands,
            });
        }
        self.upto[seat as usize] = due;
    }

    /// Host: send the batches of every seat kept open, up to `tick + delay`
    /// as for its own. The first carries the order that has the seat's
    /// machines hold where they stand; the rest are empty.
    fn stand_in(&mut self, tick: u64) {
        if !self.is_host() {
            return;
        }
        let due = tick + self.delay;
        for s in 0..self.plan.seats.len() {
            let Some(player) = self.players[s] else {
                continue;
            };
            if self.away_until[s].is_none() || self.dropped[s].is_some() {
                continue;
            }
            let from = self.upto[s] + 1;
            if from > due {
                continue;
            }
            let seat = s as u8;
            for at in from..=due {
                let mut commands = Vec::new();
                if std::mem::take(&mut self.hold_due[s]) {
                    commands.extend(hold_still(&self.canonical, player));
                }
                self.scheduled
                    .entry(at)
                    .or_default()
                    .extend(commands.iter().cloned().map(|c| (seat, c)));
                self.broadcast(&Wire::Batch {
                    seat,
                    tick: at,
                    now: tick,
                    commands,
                });
            }
            self.upto[s] = due;
        }
    }

    /// Host: keep a silent seat open for its player to come back, for up to
    /// `KEEP_OPEN`, while the others play on. It has not surrendered; its
    /// machines hold where they stand until it returns.
    pub fn keep_seat_open(&mut self, seat: u8) -> Result<(), String> {
        if !self.is_host() {
            return Err("Only the host can keep a seat open.".into());
        }
        if !self.can_keep_open(seat) {
            return Err("Only a silent player's seat can be kept open.".into());
        }
        let s = seat as usize;
        let now = Instant::now();
        if let Hub::Host {
            peers,
            gone_since,
            paths_sent,
            ..
        } = &mut self.hub
        {
            // The old connection is finished with: were it to wake, its
            // orders would land on ticks the host has already filled.
            if let Some(peer) = peers[s].take() {
                self.link.disconnect(peer);
            }
            gone_since[s].get_or_insert(now);
            *paths_sent = now
                .checked_sub(Duration::from_secs(1))
                .unwrap_or(*paths_sent);
        }
        self.away_until[s] = Some(now + self.keep_open_for);
        self.hold_due[s] = true;
        let name = self.seat_name(seat).to_string();
        self.notices.push(format!("{name}'S SEAT KEPT OPEN"));
        Ok(())
    }

    /// Whether this seat (the host) may keep `seat` open now: a player's
    /// seat, not dropped or kept open already, whose connection is silent.
    pub fn can_keep_open(&self, seat: u8) -> bool {
        let s = seat as usize;
        self.is_host()
            && s < self.plan.seats.len()
            && seat != self.local_seat
            && self.players[s].is_some()
            && self.dropped[s].is_none()
            && self.away_until[s].is_none()
            && self.ending.is_none()
            && self.canonical.outcome.is_none()
            && self.quiet_of(seat).is_some_and(|q| q >= QUIET_SHOWN)
    }

    /// A seat kept open, and how long before it is dropped: the one with the
    /// least time left.
    fn away_seat(&self) -> Option<(u8, Duration)> {
        let now = Instant::now();
        self.away_until
            .iter()
            .enumerate()
            .filter(|(s, _)| *s != self.local_seat as usize)
            .filter_map(|(s, until)| until.map(|at| (s as u8, at.saturating_duration_since(now))))
            .min_by_key(|(_, left)| *left)
    }

    fn ready_for(&self, tick: u64) -> bool {
        (0..self.plan.seats.len()).all(|s| {
            self.players[s].is_none()
                || self.dropped[s].is_some_and(|from| tick >= from)
                || self.upto[s] >= tick
        })
    }

    fn check_hashes(&mut self) {
        let local = self.local_seat as usize;
        for (tick, row) in &self.hashes {
            let Some(mine) = &row[local] else { continue };
            let differ: Vec<u8> = row
                .iter()
                .enumerate()
                .filter(|(_, h)| h.as_ref().is_some_and(|h| h != mine))
                .map(|(s, _)| s as u8)
                .collect();
            if !differ.is_empty() && self.desync.is_none() {
                self.desync = Some(*tick);
                self.ending = Some(Ending::Desync {
                    tick: *tick,
                    seats: differ,
                });
                break;
            }
        }
        let keep = self.canonical.tick.saturating_sub(HASH_EVERY * 20);
        self.hashes.retain(|tick, _| *tick >= keep);
    }

    /// Advance one canonical tick if every batch for it has arrived.
    pub fn try_step(&mut self) -> StepOutcome {
        self.pump();
        if self.ending.is_some() || self.desync.is_some() || self.canonical.outcome.is_some() {
            return StepOutcome::Ended;
        }
        if Instant::now() < self.starts_at || self.ahead_of_others() {
            return StepOutcome::Stalled;
        }
        let tick = self.canonical.tick;
        self.send_own(tick);
        self.stand_in(tick);
        if !self.ready_for(tick) {
            return StepOutcome::Stalled;
        }
        if let Some(mut batch) = self.scheduled.remove(&tick) {
            // Seat order, then arrival order within a seat: every peer
            // applies the same sequence.
            batch.sort_by_key(|(seat, _)| *seat);
            for (seat, command) in batch {
                let s = seat as usize;
                let Some(player) = self.players[s] else {
                    continue;
                };
                if self.dropped[s].is_some_and(|from| tick >= from) {
                    continue;
                }
                // A command that no longer validates at execution is logged
                // as rejected on every peer alike. This seat's own refusal
                // is kept for the field to say, because the player was told
                // the order was accepted a few ticks ago.
                if let Err(refusal) = self.canonical.issue(player, command)
                    && seat == self.local_seat
                {
                    self.rejection = Some(refusal);
                }
            }
        }
        for s in 0..self.plan.seats.len() {
            if self.dropped[s] == Some(tick)
                && let Some(player) = self.players[s]
            {
                let _ = self.canonical.issue(player, Command::Surrender);
            }
        }
        self.canonical.step();
        let now = self.canonical.tick;
        if now.is_multiple_of(HASH_EVERY) {
            let hash = self.canonical.state_hash();
            self.record_hash(self.local_seat, now, hash.clone());
            self.broadcast(&Wire::Hash {
                seat: self.local_seat,
                tick: now,
                hash,
            });
            self.review_delay();
        }
        self.check_hashes();
        StepOutcome::Stepped
    }

    /// Say goodbye to the others. The match ends for this seat; for the
    /// others this seat surrenders.
    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.broadcast(&Wire::Bye {
            seat: self.local_seat,
        });
        self.link.flush(Duration::from_millis(300));
    }

    /// Seconds since anything arrived from the peer this seat waits for.
    pub fn peer_quiet_seconds(&self) -> u64 {
        self.waiting_for()
            .map(|(_, quiet)| quiet.as_secs())
            .unwrap_or(0)
    }

    /// The seat whose batch this one is missing, and how long it has been
    /// quiet, while that is long enough to mention.
    fn waiting_for(&self) -> Option<(u8, Duration)> {
        if self.ending.is_some() || self.desync.is_some() || self.canonical.outcome.is_some() {
            return None;
        }
        let tick = self.canonical.tick;
        (0..self.plan.seats.len() as u8)
            .filter(|s| {
                let s = *s as usize;
                self.players[s].is_some()
                    && s != self.local_seat as usize
                    && self.dropped[s].is_none_or(|from| tick < from)
                    && self.upto[s] < tick
            })
            .filter_map(|s| self.quiet_of(s).map(|q| (s, q)))
            .filter(|(_, q)| *q >= QUIET_SHOWN)
            .max_by_key(|(_, q)| *q)
    }

    /// True while this seat is held up by a silent peer.
    pub fn waiting_on_peer(&self) -> bool {
        self.waiting_for().is_some()
    }

    /// The connected seat whose orders this one has waited on longest, while
    /// that is long enough to mention: it is busy or slow, not gone, and no
    /// drop clock runs for it.
    fn slow_peer(&self) -> Option<(u8, Duration)> {
        if self.ending.is_some() || self.desync.is_some() || self.canonical.outcome.is_some() {
            return None;
        }
        let tick = self.canonical.tick;
        (0..self.plan.seats.len() as u8)
            .filter(|s| {
                let s = *s as usize;
                self.players[s].is_some()
                    && s != self.local_seat as usize
                    && self.dropped[s].is_none_or(|from| tick < from)
                    && self.upto[s] < tick
            })
            .filter(|s| self.quiet_of(*s).is_none_or(|q| q < QUIET_SHOWN))
            .map(|s| {
                let late = self.heard_tick[s as usize]
                    .map_or_else(|| self.starts_at.elapsed(), |(_, at)| at.elapsed());
                (s, late)
            })
            .filter(|(_, late)| *late >= QUIET_SHOWN)
            .max_by_key(|(_, late)| *late)
    }

    /// True while this seat waits on a connected peer's late orders.
    pub fn waiting_on_slow_peer(&self) -> bool {
        self.slow_peer().is_some()
    }

    /// What the field shows about the network.
    pub fn net_view(&self) -> NetView {
        let rtt_ms = match &self.hub {
            Hub::Guest { host, .. } => self.link.stats(*host).and_then(|s| s.rtt_ms),
            Hub::Host { peers, .. } => peers
                .iter()
                .flatten()
                .filter_map(|p| self.link.stats(*p).and_then(|s| s.rtt_ms))
                .max(),
        };
        NetView {
            rtt_ms,
            delay: self.delay,
            waiting: self.waiting_for().map(|(seat, quiet)| {
                (
                    seat,
                    self.seat_name(seat).to_string(),
                    quiet.as_secs(),
                    self.drop_after.saturating_sub(quiet).as_secs(),
                )
            }),
            slow: self
                .slow_peer()
                .map(|(seat, late)| (seat, self.seat_name(seat).to_string(), late.as_secs())),
            away: self
                .away_seat()
                .map(|(seat, left)| (seat, self.seat_name(seat).to_string(), left.as_secs())),
            can_keep_open: self
                .waiting_for()
                .is_some_and(|(seat, _)| self.can_keep_open(seat)),
            watching: self.watching(),
        }
    }

    /// Things worth a line on the field, taken once.
    pub fn take_notices(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notices)
    }

    /// The refusal from this seat's own command at execution, taken once.
    pub fn take_rejection(&mut self) -> Option<String> {
        self.rejection.take()
    }

    /// This seat's and the others' hashes, newest last, for a desync
    /// post-mortem: (tick, this seat's, the first other seat's).
    pub fn hash_history(&self) -> Vec<(u64, String, Option<String>)> {
        let local = self.local_seat as usize;
        self.hashes
            .iter()
            .filter_map(|(tick, row)| {
                let mine = row[local].clone()?;
                let other = row
                    .iter()
                    .enumerate()
                    .filter(|(s, _)| *s != local)
                    .find_map(|(_, h)| h.clone());
                Some((*tick, mine, other))
            })
            .collect()
    }

    /// One line for the HUD and the agent harness.
    pub fn status(&self) -> String {
        match &self.ending {
            Some(Ending::Desync { tick, .. }) => format!("DESYNC AT TICK {tick}"),
            Some(Ending::HostLeft) => "THE HOST HAS LEFT".to_string(),
            Some(Ending::Dropped) => "YOU WERE DROPPED FROM THE MATCH".to_string(),
            None => {
                if let Some((seat, quiet)) = self.waiting_for() {
                    format!("WAITING FOR {} {}S", self.seat_name(seat), quiet.as_secs())
                } else if let Some((seat, late)) = self.slow_peer() {
                    format!("{} IS BEHIND {}S", self.seat_name(seat), late.as_secs())
                } else if let Some((seat, left)) = self.away_seat() {
                    format!("{} AWAY {}S", self.seat_name(seat), left.as_secs())
                } else if self.disconnected {
                    "THE OTHER PLAYER HAS LEFT".to_string()
                } else {
                    String::new()
                }
            }
        }
    }

    /// For tests: cut or restore this seat's outgoing packets.
    #[cfg(test)]
    pub(crate) fn set_conditions(&self, conditions: super::link::Conditions) {
        self.link.set_conditions(conditions);
    }

    /// For tests: drop a silent seat after `after` rather than a minute.
    #[cfg(test)]
    pub(crate) fn set_drop_after(&mut self, after: Duration) {
        self.drop_after = after;
    }

    /// For tests: keep a seat open for `open` rather than five minutes.
    #[cfg(test)]
    pub(crate) fn set_keep_open_for(&mut self, open: Duration) {
        self.keep_open_for = open;
    }

    /// For tests: step from now rather than at the agreed start.
    #[cfg(test)]
    pub(crate) fn start_now(&mut self) {
        self.starts_at = Instant::now();
    }

    /// For tests: reach into the canonical world, to make a desync.
    #[cfg(test)]
    pub(crate) fn canonical_mut(&mut self) -> &mut World {
        &mut self.canonical
    }

    /// For tests: vanish without a goodbye, as a crash would.
    #[cfg(test)]
    pub(crate) fn crash(mut self) {
        self.closed = true;
        self.link.set_conditions(super::link::Conditions {
            loss_percent: 100,
            ..Default::default()
        });
    }
}

/// The orders that have `player`'s machines hold where they stand while its
/// seat is kept open: every machine that moves, workers too, so the seat
/// neither fights nor gathers until its player is back. A machine aboard a
/// transport takes no orders and stays aboard. In groups no larger than the
/// simulation takes in one order.
fn hold_still(world: &World, player: u8) -> Vec<Command> {
    const MOST_IN_ONE_ORDER: usize = 128;
    let mut units: Vec<u32> = world
        .entities
        .iter()
        .filter(|e| e.owner == player && e.hp > 0 && e.aboard.is_none() && !e.kind.is_building())
        .map(|e| e.id)
        .collect();
    units.sort_unstable();
    units
        .chunks(MOST_IN_ONE_ORDER)
        .map(|chunk| Command::Hold {
            units: chunk.to_vec(),
        })
        .collect()
}
