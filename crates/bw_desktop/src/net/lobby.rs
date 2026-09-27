//! The lobby: find each other, pick sides, get ready, start together.
//!
//! The host opens a port, asks the router to forward it (UPnP) and asks a
//! STUN server what its public address is, then shows a join code built
//! from what it learned. A guest pastes the code and tries every address in
//! it at once. If none answers within a few seconds the guest shows a reply
//! code of its own; the host pastes that, and both send towards each other
//! until their routers let the packets through.
//!
//! Seats are listed in order. Each playing seat picks a side; the map
//! decides how many can play and everyone else watches. The host starts
//! the match when every player is ready, and every seat begins stepping at
//! the same moment, give or take half a round trip.

use super::code::{self, JoinCode};
use super::link::{Conditions, Endpoint, LinkEvent, PeerId, random_id};
use super::protocol::{self, MatchPlan, SeatInfo, Wire};
use super::session::{self, Session, Start};
use super::upnp::{self, PortMapping};
use bw_core::Faction;
use bw_sim::MapId;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

/// How long a host started from the command line waits for its guest.
pub const ACCEPT_WAIT: Duration = Duration::from_secs(120);
/// How long a guest keeps trying to reach a host.
pub const REACH_FOR: Duration = Duration::from_secs(120);
/// A guest unanswered this long shows a reply code.
const REPLY_AFTER: Duration = Duration::from_secs(3);
/// The host's code waits this long for the router and STUN to answer.
const CODE_WAIT: Duration = Duration::from_millis(3500);
/// A match starts this long after the host presses start.
const START_IN: Duration = Duration::from_millis(3200);
const MAX_SEATS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Host: open for guests.
    Open,
    /// Guest: trying to reach the host.
    Reaching,
    /// Guest: in the host's lobby.
    Seated,
    /// Over: the reason, for the screen.
    Failed(String),
}

/// How a lobby should behave; the defaults suit a person at the screen.
#[derive(Clone, Debug)]
pub struct LobbyOptions {
    pub name: String,
    pub conditions: Conditions,
    /// A fixed input delay, instead of one sized to the connection.
    pub fixed_delay: Option<u64>,
    /// Ready at once, and a host starts as soon as the map is full: for the
    /// command line and the agent harness.
    pub auto: bool,
    /// Ask a STUN server for the public address.
    pub stun: bool,
    /// Ask the router to forward the port.
    pub upnp: bool,
    /// With `auto`, the side a guest asks for when it is seated (the
    /// harness's `--join ... --faction F`); otherwise the side it is given.
    pub prefer: Option<Faction>,
}

impl Default for LobbyOptions {
    fn default() -> Self {
        LobbyOptions {
            name: protocol::local_name(),
            conditions: Conditions::from_env(),
            fixed_delay: None,
            auto: false,
            stun: true,
            upnp: true,
            prefer: None,
        }
    }
}

impl LobbyOptions {
    /// For a match on this machine: nothing leaves it.
    pub fn local() -> LobbyOptions {
        LobbyOptions {
            stun: false,
            upnp: false,
            ..LobbyOptions::default()
        }
    }
}

pub struct Lobby {
    link: Option<Endpoint>,
    is_host: bool,
    room: u32,
    pub seats: Vec<SeatInfo>,
    /// Host: each seat's connection (none for its own).
    peers: Vec<Option<PeerId>>,
    tokens: Vec<u64>,
    strangers: Vec<(PeerId, Instant)>,
    /// Guest: the host once reached, and the attempt before that.
    host_peer: Option<PeerId>,
    attempt: Option<PeerId>,
    /// This seat, once the host has said.
    pub you: Option<u8>,
    pub seed: u64,
    /// The map the host picked.
    pub map: MapId,
    pub phase: Phase,
    mapping: Option<PortMapping>,
    public: Option<SocketAddr>,
    lan: Option<SocketAddr>,
    code: Option<String>,
    opened: Instant,
    reply: Option<String>,
    /// Guest: the token for coming back to this match.
    pub token: Option<u64>,
    rejoin: Option<u64>,
    options: LobbyOptions,
    roster_sent: Instant,
    notices: Vec<String>,
}

fn seat(name: &str, faction: Option<Faction>) -> SeatInfo {
    SeatInfo {
        name: name.to_string(),
        faction,
        ready: false,
        rtt_ms: None,
    }
}

fn distinct_name(seats: &[SeatInfo], name: &str) -> String {
    if !seats.iter().any(|s| s.name == name) {
        return name.to_string();
    }
    (2..)
        .map(|n| format!("{} {n}", &name[..name.len().min(8)]))
        .find(|candidate| !seats.iter().any(|s| s.name == *candidate))
        .expect("a free name")
}

impl Lobby {
    /// Open a lobby on `bind` (`0.0.0.0:47800` for the internet).
    pub fn host(
        bind: &str,
        faction: Faction,
        seed: u64,
        options: LobbyOptions,
    ) -> Result<Lobby, String> {
        let link = match Endpoint::bind(bind, options.conditions) {
            Ok(link) => link,
            // The usual port is taken (another game open): any port will do,
            // since the code carries it.
            Err(_) if bind.ends_with(&format!(":{}", code::DEFAULT_PORT)) => {
                let any = bind.replace(&format!(":{}", code::DEFAULT_PORT), ":0");
                Endpoint::bind(any.as_str(), options.conditions)?
            }
            Err(e) => return Err(e),
        };
        let room = (random_id() as u32).max(1);
        link.listen(room);
        let local = link.local_addr();
        let lan = if local.ip().is_unspecified() {
            code::local_network_address().map(|ip| SocketAddr::new(IpAddr::V4(ip), local.port()))
        } else {
            Some(local)
        };
        let outward = local.ip().is_unspecified();
        if options.stun && outward {
            link.discover_mapping(super::stun::SERVERS);
        }
        let mapping = (options.upnp && outward).then(|| PortMapping::request(local.port()));
        Ok(Lobby {
            link: Some(link),
            is_host: true,
            room,
            seats: vec![seat(&options.name, Some(faction))],
            peers: vec![None],
            tokens: vec![0],
            strangers: Vec::new(),
            host_peer: None,
            attempt: None,
            you: Some(0),
            seed,
            map: MapId::default(),
            phase: Phase::Open,
            mapping,
            public: None,
            lan,
            code: None,
            opened: Instant::now(),
            reply: None,
            token: None,
            rejoin: None,
            options,
            roster_sent: Instant::now(),
            notices: Vec::new(),
        })
    }

    /// Start reaching the host named by `join`. `rejoin` is the token from
    /// a match this game left unfinished.
    pub fn join(
        join: &JoinCode,
        rejoin: Option<u64>,
        options: LobbyOptions,
    ) -> Result<Lobby, String> {
        if join.kind == code::CodeKind::Reply {
            return Err(
                "That is a reply code: the host adds it. Paste the host's code here.".into(),
            );
        }
        let local_only = join.addresses.iter().all(|a| a.ip().is_loopback());
        let bind = if local_only {
            "127.0.0.1:0"
        } else {
            "0.0.0.0:0"
        };
        let link = Endpoint::bind(bind, options.conditions)?;
        let attempt = link.connect(&join.addresses, join.room, REACH_FOR);
        let local = link.local_addr();
        let lan = if local.ip().is_unspecified() {
            code::local_network_address().map(|ip| SocketAddr::new(IpAddr::V4(ip), local.port()))
        } else {
            Some(local)
        };
        if options.stun && !local_only {
            link.discover_mapping(super::stun::SERVERS);
        }
        Ok(Lobby {
            link: Some(link),
            is_host: false,
            room: join.room,
            seats: Vec::new(),
            peers: Vec::new(),
            tokens: Vec::new(),
            strangers: Vec::new(),
            host_peer: None,
            attempt: Some(attempt),
            you: None,
            seed: 0,
            map: MapId::default(),
            phase: Phase::Reaching,
            mapping: None,
            public: None,
            lan,
            code: None,
            opened: Instant::now(),
            reply: None,
            token: None,
            rejoin,
            options,
            roster_sent: Instant::now(),
            notices: Vec::new(),
        })
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.link.as_ref().map(Endpoint::local_addr)
    }

    /// Host: the join code, once the router and STUN have had their say.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Guest: the code to send back, once reaching the host is taking long.
    pub fn reply_code(&self) -> Option<&str> {
        self.reply.as_deref()
    }

    /// Host: whether UPnP forwarded the port, once known.
    pub fn forwarded(&mut self) -> Option<bool> {
        match self.mapping.as_mut()?.outcome()? {
            upnp::Outcome::Mapped { .. } => Some(true),
            upnp::Outcome::Unavailable(_) => Some(false),
        }
    }

    pub fn take_notices(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notices)
    }

    pub fn rtt_ms(&self) -> Option<u32> {
        let link = self.link.as_ref()?;
        self.host_peer
            .and_then(|p| link.stats(p))
            .and_then(|s| s.rtt_ms)
    }

    fn send(&self, peer: PeerId, message: &Wire) {
        if let Some(link) = &self.link {
            let _ = link.send(peer, &message.encode());
        }
    }

    /// Host: tell each guest the lobby as it stands.
    fn send_roster(&mut self) {
        self.roster_sent = Instant::now();
        for (index, peer) in self.peers.iter().enumerate() {
            if let Some(peer) = peer {
                self.send(
                    *peer,
                    &Wire::Lobby {
                        seats: self.seats.clone(),
                        seed: self.seed,
                        you: index as u8,
                        map: self.map,
                    },
                );
            }
        }
    }

    fn remove_seat(&mut self, index: usize) {
        if index == 0 || index >= self.seats.len() {
            return;
        }
        let name = self.seats.remove(index).name;
        self.peers.remove(index);
        self.tokens.remove(index);
        self.notices.push(format!("{name} LEFT"));
        self.send_roster();
    }

    /// Everything the network brought. Returns the match once it starts.
    pub fn poll(&mut self) -> Option<Session> {
        let now = Instant::now();
        while let Some(event) = self.link.as_ref().and_then(Endpoint::poll) {
            match event {
                LinkEvent::Connected { peer, .. } => {
                    if self.is_host {
                        self.strangers.push((peer, now));
                    } else if Some(peer) == self.attempt {
                        self.host_peer = Some(peer);
                        self.attempt = None;
                        self.send(
                            peer,
                            &Wire::Hello {
                                protocol: protocol::PROTOCOL,
                                rules_digest: bw_content::rules_digest(),
                                build: bw_content::build_fingerprint().to_string(),
                                name: self.options.name.clone(),
                                rejoin: self.rejoin,
                            },
                        );
                    }
                }
                LinkEvent::Message { peer, bytes } => {
                    if let Ok(message) = Wire::decode(&bytes)
                        && let Some(session) = self.handle(peer, message)
                    {
                        return Some(session);
                    }
                }
                LinkEvent::Closed { peer } => {
                    if self.is_host {
                        if let Some(index) = self.peers.iter().position(|p| *p == Some(peer)) {
                            self.remove_seat(index);
                        }
                    } else if Some(peer) == self.host_peer {
                        self.phase = Phase::Failed("The host closed the lobby.".into());
                    }
                }
                LinkEvent::ConnectFailed { peer } => {
                    if Some(peer) == self.attempt {
                        self.phase = Phase::Failed(
                            "Couldn't reach the host. Check the code, or ask the host to forward UDP port 47800.".into(),
                        );
                    }
                }
                LinkEvent::Mapped(addr) => {
                    self.public.get_or_insert(addr);
                }
            }
            if matches!(self.phase, Phase::Failed(_)) {
                return None;
            }
        }
        if self.is_host {
            self.host_timers(now)
        } else {
            if self.phase == Phase::Reaching
                && self.reply.is_none()
                && now.duration_since(self.opened) >= REPLY_AFTER
            {
                let addresses: Vec<SocketAddr> = self.public.into_iter().chain(self.lan).collect();
                if !addresses.is_empty() {
                    self.reply = Some(JoinCode::reply(self.room, &addresses));
                }
            }
            None
        }
    }

    fn host_timers(&mut self, now: Instant) -> Option<Session> {
        let _ = self.forwarded();
        if self.code.is_none() {
            let mapped = self.mapping.as_mut().and_then(|m| m.outcome().cloned());
            let waited = now.duration_since(self.opened) >= CODE_WAIT;
            let upnp_done = self.mapping.is_none() || mapped.is_some();
            let stun_done = !self.options.stun || self.public.is_some();
            if waited || (upnp_done && stun_done) {
                let mut addresses = Vec::new();
                if let Some(upnp::Outcome::Mapped { external }) = mapped {
                    addresses.push(external);
                    self.public.get_or_insert(external);
                }
                addresses.extend(self.public);
                addresses.extend(self.lan);
                if addresses.is_empty()
                    && let Some(link) = &self.link
                {
                    addresses.push(link.local_addr());
                }
                self.code = Some(JoinCode::host(self.room, &addresses));
            }
        }
        // A connection that never introduces itself goes.
        let stale: Vec<PeerId> = self
            .strangers
            .iter()
            .filter(|(_, at)| now.duration_since(*at) > Duration::from_secs(10))
            .map(|(p, _)| *p)
            .collect();
        self.strangers
            .retain(|(_, at)| now.duration_since(*at) <= Duration::from_secs(10));
        if let Some(link) = &self.link {
            for peer in stale {
                link.disconnect(peer);
            }
        }
        if now.duration_since(self.roster_sent) >= Duration::from_secs(1) {
            let rtts: Vec<Option<u32>> = self
                .peers
                .iter()
                .map(|p| {
                    p.and_then(|p| self.link.as_ref()?.stats(p))
                        .and_then(|s| s.rtt_ms)
                })
                .collect();
            for (seat, rtt) in self.seats.iter_mut().zip(rtts) {
                seat.rtt_ms = rtt;
            }
            self.send_roster();
        }
        if self.options.auto && self.can_start().is_ok() {
            return self.start().ok();
        }
        None
    }

    fn handle(&mut self, peer: PeerId, message: Wire) -> Option<Session> {
        if self.is_host {
            self.handle_as_host(peer, message);
            None
        } else if Some(peer) == self.host_peer {
            self.handle_as_guest(peer, message)
        } else {
            None
        }
    }

    fn refuse(&self, peer: PeerId, reason: &str) {
        if let Some(link) = &self.link {
            let _ = link.send(
                peer,
                &Wire::Refuse {
                    reason: reason.to_string(),
                }
                .encode(),
            );
            link.flush(Duration::from_millis(200));
            link.disconnect(peer);
        }
    }

    fn handle_as_host(&mut self, peer: PeerId, message: Wire) {
        let seat = self.peers.iter().position(|p| *p == Some(peer));
        match message {
            Wire::Hello {
                protocol,
                rules_digest,
                build,
                name,
                ..
            } if seat.is_none() => {
                self.strangers.retain(|(p, _)| *p != peer);
                if let Err(reason) = session::check_hello(protocol, &rules_digest, &build) {
                    self.refuse(peer, &reason);
                    return;
                }
                if self.seats.len() >= MAX_SEATS {
                    self.refuse(peer, "The lobby is full.");
                    return;
                }
                let name = distinct_name(&self.seats, &protocol::clean_name(&name));
                let faction = protocol::free_side(&self.seats, self.map);
                self.seats.push(seat_info(&name, faction));
                self.peers.push(Some(peer));
                let token = random_id();
                self.tokens.push(token);
                self.send(
                    peer,
                    &Wire::Welcome {
                        seat: (self.seats.len() - 1) as u8,
                        token,
                    },
                );
                self.notices.push(format!("{name} JOINED"));
                self.send_roster();
            }
            Wire::Choose { faction, ready } => {
                let Some(index) = seat else { return };
                if self.seats[index].faction != faction {
                    protocol::settle_sides(&mut self.seats, index, faction, self.map);
                }
                if self.seats[index].faction == faction {
                    self.seats[index].ready = ready;
                }
                self.send_roster();
            }
            Wire::Bye { .. } => {
                if let Some(index) = seat {
                    self.remove_seat(index);
                    if let Some(link) = &self.link {
                        link.disconnect(peer);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_as_guest(&mut self, peer: PeerId, message: Wire) -> Option<Session> {
        match message {
            Wire::Welcome { seat, token } => {
                self.you = Some(seat);
                self.token = Some(token);
                self.phase = Phase::Seated;
            }
            Wire::Lobby {
                seats,
                seed,
                you,
                map,
            } => {
                let first = self.seats.is_empty();
                self.seats = seats;
                self.seed = seed;
                self.map = map;
                self.you = Some(you);
                self.phase = Phase::Seated;
                if self.options.auto && (first || !self.seats[you as usize].ready) {
                    let given = self.seats[you as usize].faction;
                    let faction = if first {
                        self.options.prefer.or(given)
                    } else {
                        given
                    };
                    self.send(
                        peer,
                        &Wire::Choose {
                            faction,
                            ready: true,
                        },
                    );
                }
            }
            Wire::Refuse { reason } => {
                self.phase = Phase::Failed(reason);
            }
            Wire::Start {
                plan,
                start_in_ms,
                delay,
            } => {
                let you = self.you?;
                let half_trip = Duration::from_millis(u64::from(self.rtt_ms().unwrap_or(0) / 2));
                let starts_at = Instant::now() + Duration::from_millis(u64::from(start_in_ms))
                    - half_trip.min(Duration::from_millis(u64::from(start_in_ms)));
                let start = Start {
                    link: self.link.take()?,
                    plan,
                    local_seat: you,
                    delay: self.options.fixed_delay.unwrap_or(delay),
                    fixed_delay: self.options.fixed_delay.is_some(),
                    starts_at,
                    port_mapping: None,
                };
                return match Session::guest(start, peer) {
                    Ok(session) => Some(session),
                    Err(reason) => {
                        self.phase = Phase::Failed(reason);
                        None
                    }
                };
            }
            Wire::Resume {
                seat,
                plan,
                world,
                batches,
                upto,
                dropped,
                delay,
            } => {
                let link = self.link.take()?;
                return match Session::resume(
                    link, peer, seat, plan, *world, batches, upto, dropped, delay,
                ) {
                    Ok(session) => Some(session),
                    Err(reason) => {
                        self.phase = Phase::Failed(reason);
                        None
                    }
                };
            }
            _ => {}
        }
        None
    }

    /// Pick a side for this seat, or `None` to watch.
    pub fn choose(&mut self, faction: Option<Faction>) {
        let Some(you) = self.you.map(usize::from) else {
            return;
        };
        if you >= self.seats.len() {
            return;
        }
        if self.is_host {
            protocol::settle_sides(&mut self.seats, you, faction, self.map);
            self.send_roster();
        } else if let Some(host) = self.host_peer {
            // Shown at once; the host's roster has the last word.
            protocol::settle_sides(&mut self.seats, you, faction, self.map);
            self.send(
                host,
                &Wire::Choose {
                    faction,
                    ready: false,
                },
            );
        }
    }

    pub fn set_ready(&mut self, ready: bool) {
        let Some(you) = self.you.map(usize::from) else {
            return;
        };
        if you >= self.seats.len() {
            return;
        }
        self.seats[you].ready = ready;
        if self.is_host {
            self.send_roster();
        } else if let Some(host) = self.host_peer {
            let faction = self.seats[you].faction;
            self.send(host, &Wire::Choose { faction, ready });
        }
    }

    pub fn ready(&self) -> bool {
        self.you
            .and_then(|y| self.seats.get(y as usize))
            .is_some_and(|s| s.ready)
    }

    /// Host: play `map`. Seats beyond its players watch.
    pub fn set_map(&mut self, map: MapId) {
        if self.is_host && self.map != map {
            self.map = map;
            protocol::fit_sides(&mut self.seats, map);
            self.send_roster();
        }
    }

    /// Host: send towards a guest that could not reach us.
    pub fn add_reply(&mut self, text: &str) -> Result<(), String> {
        let reply = JoinCode::parse(text)?;
        if reply.kind == code::CodeKind::Host {
            return Err("That is a host code. Paste the code your guest sent back.".into());
        }
        let link = self.link.as_ref().ok_or("The lobby is closed.")?;
        link.punch(&reply.addresses, self.room, Duration::from_secs(60));
        Ok(())
    }

    /// Host: why the match cannot start yet, if it cannot.
    pub fn can_start(&self) -> Result<(), String> {
        if !self.is_host {
            return Err("The host starts the match.".into());
        }
        let plan = MatchPlan {
            seed: self.seed,
            seats: self.seats.clone(),
            map: self.map,
        };
        plan.check()?;
        if let Some(waiting) = self
            .seats
            .iter()
            .skip(1)
            .find(|s| s.faction.is_some() && !s.ready)
        {
            return Err(format!("Waiting for {} to be ready.", waiting.name));
        }
        Ok(())
    }

    /// Host: start the match for everyone.
    pub fn start(&mut self) -> Result<Session, String> {
        self.can_start()?;
        let link = self.link.take().ok_or("The lobby is closed.")?;
        for peer in self.strangers.drain(..).map(|(p, _)| p) {
            link.disconnect(peer);
        }
        let mut seats = self.seats.clone();
        for seat in &mut seats {
            seat.ready = true;
        }
        let plan = MatchPlan {
            seed: self.seed,
            seats,
            map: self.map,
        };
        let slowest = self
            .peers
            .iter()
            .flatten()
            .filter_map(|p| link.stats(*p).and_then(|s| s.rtt_ms))
            .max()
            .unwrap_or(0);
        let delay = self.options.fixed_delay.unwrap_or_else(|| {
            ((f64::from(slowest) / 2.0 + 20.0) / (1000.0 / 30.0))
                .ceil()
                .clamp(session::MIN_DELAY as f64, session::MAX_DELAY as f64) as u64
        });
        let message = Wire::Start {
            plan: plan.clone(),
            start_in_ms: START_IN.as_millis() as u32,
            delay,
        }
        .encode();
        for peer in self.peers.iter().flatten() {
            let _ = link.send(*peer, &message);
        }
        let start = Start {
            link,
            plan,
            local_seat: 0,
            delay,
            fixed_delay: self.options.fixed_delay.is_some(),
            starts_at: Instant::now() + START_IN,
            port_mapping: self.mapping.take(),
        };
        let session = Session::host(start, self.peers.clone(), self.tokens.clone());
        if let Err(reason) = &session {
            self.phase = Phase::Failed(reason.clone());
        }
        session
    }

    /// Leave, telling the others.
    pub fn leave(mut self) {
        if let Some(link) = self.link.take() {
            let bye = Wire::Bye {
                seat: self.you.unwrap_or(0),
            }
            .encode();
            let peers: Vec<PeerId> = if self.is_host {
                self.peers.iter().flatten().copied().collect()
            } else {
                self.host_peer.into_iter().collect()
            };
            for peer in &peers {
                let _ = link.send(*peer, &bye);
            }
            link.flush(Duration::from_millis(200));
        }
    }
}

fn seat_info(name: &str, faction: Option<Faction>) -> SeatInfo {
    seat(name, faction)
}

/// Host a match on `map` from the command line: wait until every side of
/// the map is taken, then start at once.
pub fn host_blocking(
    bind: &str,
    seed: u64,
    faction: Faction,
    map: MapId,
    delay: Option<u64>,
    wait: Duration,
) -> Result<Session, String> {
    let options = LobbyOptions {
        fixed_delay: delay,
        auto: true,
        ..LobbyOptions::local()
    };
    let mut lobby = Lobby::host(bind, faction, seed, options)?;
    lobby.set_map(map);
    if let Some(addr) = lobby.local_addr() {
        println!("Listening on UDP {addr}.");
    }
    wait_for(&mut lobby, wait, "no other player connected")
}

/// Join a match from the command line with a code or an address.
pub fn join_blocking(
    target: &str,
    wait: Duration,
    prefer: Option<Faction>,
) -> Result<Session, String> {
    let code = JoinCode::parse(target)?;
    let options = LobbyOptions {
        auto: true,
        prefer,
        ..LobbyOptions::local()
    };
    let mut lobby = Lobby::join(&code, None, options)?;
    wait_for(&mut lobby, wait, "the host did not answer")
}

pub(super) fn wait_for(lobby: &mut Lobby, wait: Duration, what: &str) -> Result<Session, String> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if let Some(session) = lobby.poll() {
            return Ok(session);
        }
        if let Phase::Failed(reason) = &lobby.phase {
            return Err(reason.clone());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("{what} within {} seconds", wait.as_secs()))
}
