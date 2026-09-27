//! Reliable, ordered messages between peers over one UDP socket.
//!
//! Lockstep needs every batch, in order, and soon: a batch that arrives late
//! stalls every seat. TCP delivers in order but waits out a retransmission
//! timeout (200 ms or more) on a lost packet, and it cannot open a path
//! through two home routers. This link is small instead:
//!
//! - One UDP socket per game. The address a STUN server saw is the address
//!   the other player reaches, and both sides can punch a hole in their
//!   routers by sending to each other at once.
//! - Every message is numbered, and every packet acknowledges what arrived:
//!   a cumulative number and a 32-message window beyond it.
//! - Each packet carries, besides its new data, the small messages not yet
//!   acknowledged, so a lost packet is usually repaired by the next one a
//!   tick later without waiting for a timeout. Larger ones are resent on a
//!   timeout measured from the round trip.
//! - A connection is named by a random id, not by an address, so a player
//!   whose address changes mid-match (a router rebinding, Wi-Fi to cable)
//!   carries on.
//!
//! A background thread receives, acknowledges, resends and keeps the path
//! open; the game thread sends directly, so an order leaves the moment the
//! tick that carries it is stepped.
//!
//! `Conditions` delays, reorders and drops this endpoint's outgoing packets
//! on purpose, so a bad connection can be tested on one machine.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::{BuildHasher, Hasher};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub type PeerId = u64;

const MAGIC: [u8; 2] = *b"BW";
const VERSION: u8 = 1;
const KIND_CONNECT: u8 = 1;
const KIND_ACCEPT: u8 = 2;
const KIND_DATA: u8 = 3;
const KIND_PUNCH: u8 = 4;
const KIND_CLOSE: u8 = 5;

/// Payload budget per datagram. Comfortably under the 1280-byte IPv6
/// minimum MTU, so nothing is fragmented on the way.
const PACKET_BUDGET: usize = 1200;
/// Largest piece of one message in a datagram.
const SEGMENT_BYTES: usize = 1100;
/// Messages sent but not yet acknowledged, at most. A snapshot for a player
/// rejoining is the only thing that ever fills this.
const WINDOW: usize = 256;
/// A message this small is repeated in every packet until acknowledged.
const REDUNDANT_BYTES: usize = 320;
/// ...but not more often than this.
const REDUNDANT_EVERY: Duration = Duration::from_millis(12);
/// With nothing to say, a packet still goes out this often: it measures the
/// round trip, shows the peer is alive and keeps the routers' mappings open.
/// A data packet's body before its segments: connection id, stamp, echo,
/// hold time, cumulative ack, selective-ack bits and segment count.
const DATA_HEADER: usize = 8 + 4 + 4 + 2 + 4 + 4 + 1;
const KEEPALIVE: Duration = Duration::from_millis(200);
/// An acknowledgement owed is sent within this if no data carries it first.
const ACK_DELAY: Duration = Duration::from_millis(4);
/// Connection attempts and punches repeat this often.
const RETRY_EVERY: Duration = Duration::from_millis(100);
/// A peer silent this long is forgotten. The session's own, shorter grace
/// decides when a player is dropped; this only frees the slot.
const FORGET_AFTER: Duration = Duration::from_secs(600);
const MIN_RTO_MS: f64 = 30.0;
const MAX_RTO_MS: f64 = 1000.0;

static CHOSEN: std::sync::OnceLock<Conditions> = std::sync::OnceLock::new();

/// Deliberately bad network on this endpoint's outgoing packets, for tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Conditions {
    /// One-way delay added to every packet.
    pub latency_ms: u32,
    /// Up to this much more, uniformly: packets overtake one another.
    pub jitter_ms: u32,
    /// Percent of packets dropped.
    pub loss_percent: u32,
}

impl Conditions {
    /// `latency=80,jitter=30,loss=10`; any part may be left out.
    pub fn parse(text: &str) -> Result<Conditions, String> {
        let mut conditions = Conditions::default();
        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("network conditions: `{part}` is not key=value"))?;
            let value: u32 = value
                .trim()
                .trim_end_matches("ms")
                .trim_end_matches('%')
                .parse()
                .map_err(|_| format!("network conditions: `{part}` is not a number"))?;
            match key.trim() {
                "latency" | "delay" => conditions.latency_ms = value,
                "jitter" => conditions.jitter_ms = value,
                "loss" => conditions.loss_percent = value.min(100),
                other => return Err(format!("network conditions: unknown `{other}`")),
            }
        }
        Ok(conditions)
    }

    /// Set by `--net-sim`, else from `BRINEWAKE_NET_SIM`, else none.
    pub fn from_env() -> Conditions {
        if let Some(conditions) = CHOSEN.get() {
            return *conditions;
        }
        std::env::var("BRINEWAKE_NET_SIM")
            .ok()
            .and_then(|text| Conditions::parse(&text).ok())
            .unwrap_or_default()
    }

    /// Use these for every endpoint this process opens from now on.
    pub fn choose(self) {
        let _ = CHOSEN.set(self);
    }

    fn active(&self) -> bool {
        *self != Conditions::default()
    }
}

/// How the path to one peer is doing, for the lobby and the field.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // The counters and the address are for tests and logs.
pub struct PeerStats {
    pub addr: SocketAddr,
    /// Smoothed round trip, or `None` before the first measurement.
    pub rtt_ms: Option<u32>,
    /// Smoothed deviation of the round trip.
    pub jitter_ms: u32,
    /// Time since anything arrived from the peer.
    pub quiet: Duration,
    pub packets_sent: u64,
    pub segments_resent: u64,
}

#[derive(Debug)]
#[allow(dead_code)] // `addr` is for logs.
pub enum LinkEvent {
    /// A peer connected: to this endpoint, or this endpoint to it.
    Connected { peer: PeerId, addr: SocketAddr },
    /// One whole message from a peer, in the order it was sent.
    Message { peer: PeerId, bytes: Vec<u8> },
    /// The peer said goodbye, or was silent for ten minutes.
    Closed { peer: PeerId },
    /// A connection attempt reached nobody in time.
    ConnectFailed { peer: PeerId },
    /// The address a STUN server saw this socket's packets come from.
    Mapped(SocketAddr),
}

struct Segment {
    seq: u32,
    more: bool,
    bytes: Vec<u8>,
    sent_at: Option<Instant>,
    sends: u32,
}

struct Peer {
    addr: SocketAddr,
    next_seq: u32,
    unacked: VecDeque<Segment>,
    recv_next: u32,
    recv_buf: BTreeMap<u32, (bool, Vec<u8>)>,
    assembling: Vec<u8>,
    ack_owed: Option<Instant>,
    last_sent: Instant,
    last_heard: Instant,
    srtt_ms: Option<f64>,
    rttvar_ms: f64,
    /// The peer's newest stamp and when it arrived, echoed back so the peer
    /// can measure the round trip.
    echo: Option<(u32, Instant)>,
    packets_sent: u64,
    segments_resent: u64,
}

impl Peer {
    fn new(addr: SocketAddr, now: Instant) -> Peer {
        Peer {
            addr,
            next_seq: 0,
            unacked: VecDeque::new(),
            recv_next: 0,
            recv_buf: BTreeMap::new(),
            assembling: Vec::new(),
            ack_owed: None,
            last_sent: now,
            last_heard: now,
            srtt_ms: None,
            rttvar_ms: 0.0,
            echo: None,
            packets_sent: 0,
            segments_resent: 0,
        }
    }

    fn rto(&self) -> Duration {
        let ms = match self.srtt_ms {
            Some(srtt) => (srtt + 4.0 * self.rttvar_ms).clamp(MIN_RTO_MS, MAX_RTO_MS),
            None => 250.0,
        };
        Duration::from_micros((ms * 1000.0) as u64)
    }

    fn sample_rtt(&mut self, ms: f64) {
        match self.srtt_ms {
            None => {
                self.srtt_ms = Some(ms);
                self.rttvar_ms = ms / 2.0;
            }
            Some(srtt) => {
                self.rttvar_ms = 0.75 * self.rttvar_ms + 0.25 * (srtt - ms).abs();
                self.srtt_ms = Some(0.875 * srtt + 0.125 * ms);
            }
        }
    }

    /// Something in the queue wants to go out now.
    fn wants_send(&self, now: Instant) -> bool {
        let rto = self.rto();
        self.unacked.iter().take(WINDOW).any(|s| match s.sent_at {
            None => true,
            Some(at) => now.duration_since(at) >= rto,
        })
    }
}

struct Attempt {
    peer: PeerId,
    room: u32,
    candidates: Vec<SocketAddr>,
    next: Instant,
    until: Instant,
}

struct Punch {
    room: u32,
    candidates: Vec<SocketAddr>,
    next: Instant,
    until: Instant,
}

struct Shared {
    epoch: Instant,
    /// The room this endpoint accepts connections for, once it listens.
    /// Room 0, a bare address, is accepted by any listener.
    room: Option<u32>,
    peers: HashMap<PeerId, Peer>,
    attempts: Vec<Attempt>,
    punches: Vec<Punch>,
    conditions: Conditions,
    rng: u64,
    delayed: Vec<(Instant, SocketAddr, Vec<u8>)>,
    stun_ids: Vec<[u8; 12]>,
    events: Sender<LinkEvent>,
    closing: bool,
}

impl Shared {
    fn stamp(&self, now: Instant) -> u32 {
        // Zero means "no stamp to echo".
        (now.duration_since(self.epoch).as_millis() as u32).max(1)
    }

    fn roll(&mut self) -> u64 {
        // xorshift64*: only for the simulated network.
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Put one datagram on the wire, through the simulated network if one
    /// is set.
    fn transmit(&mut self, socket: &UdpSocket, addr: SocketAddr, bytes: Vec<u8>, now: Instant) {
        if !self.conditions.active() {
            let _ = socket.send_to(&bytes, addr);
            return;
        }
        if (self.roll() % 100) < u64::from(self.conditions.loss_percent) {
            return;
        }
        let jitter = if self.conditions.jitter_ms > 0 {
            self.roll() % (u64::from(self.conditions.jitter_ms) + 1)
        } else {
            0
        };
        let due = now + Duration::from_millis(u64::from(self.conditions.latency_ms) + jitter);
        self.delayed.push((due, addr, bytes));
    }

    fn flush_delayed(&mut self, socket: &UdpSocket, now: Instant) {
        let mut index = 0;
        while index < self.delayed.len() {
            if self.delayed[index].0 <= now {
                let (_, addr, bytes) = self.delayed.swap_remove(index);
                let _ = socket.send_to(&bytes, addr);
            } else {
                index += 1;
            }
        }
    }

    /// Build and send one data packet to `peer` if it has anything to carry,
    /// or always when `force`. Returns whether a packet went out.
    fn send_data(&mut self, socket: &UdpSocket, id: PeerId, now: Instant, force: bool) -> bool {
        let stamp = self.stamp(now);
        let Some(peer) = self.peers.get_mut(&id) else {
            return false;
        };
        let mut packet = Vec::with_capacity(PACKET_BUDGET);
        packet.extend_from_slice(&MAGIC);
        packet.push(VERSION);
        packet.push(KIND_DATA);
        packet.extend_from_slice(&id.to_be_bytes());
        packet.extend_from_slice(&stamp.to_be_bytes());
        let (echo, held) = match peer.echo {
            Some((echo, at)) => (echo, now.duration_since(at).as_millis().min(60_000) as u16),
            None => (0, 0),
        };
        packet.extend_from_slice(&echo.to_be_bytes());
        packet.extend_from_slice(&held.to_be_bytes());
        packet.extend_from_slice(&peer.recv_next.to_be_bytes());
        let mut bits = 0u32;
        for i in 0..32u32 {
            if peer.recv_buf.contains_key(&(peer.recv_next + 1 + i)) {
                bits |= 1 << i;
            }
        }
        packet.extend_from_slice(&bits.to_be_bytes());
        let count_at = packet.len();
        packet.push(0);
        let rto = peer.rto();
        let mut count = 0u8;
        for segment in peer.unacked.iter_mut().take(WINDOW) {
            let due = match segment.sent_at {
                None => true,
                Some(at) => {
                    let age = now.duration_since(at);
                    age >= rto || (segment.bytes.len() <= REDUNDANT_BYTES && age >= REDUNDANT_EVERY)
                }
            };
            if !due {
                continue;
            }
            if packet.len() + 7 + segment.bytes.len() > PACKET_BUDGET || count == u8::MAX {
                break;
            }
            packet.extend_from_slice(&segment.seq.to_be_bytes());
            packet.push(u8::from(segment.more));
            packet.extend_from_slice(&(segment.bytes.len() as u16).to_be_bytes());
            packet.extend_from_slice(&segment.bytes);
            if segment.sent_at.is_some() {
                peer.segments_resent += 1;
            }
            segment.sent_at = Some(now);
            segment.sends += 1;
            count += 1;
        }
        if count == 0 && !force {
            return false;
        }
        packet[count_at] = count;
        peer.ack_owed = None;
        peer.last_sent = now;
        peer.packets_sent += 1;
        let addr = peer.addr;
        self.transmit(socket, addr, packet, now);
        true
    }

    fn control(
        &mut self,
        socket: &UdpSocket,
        addr: SocketAddr,
        kind: u8,
        body: &[u8],
        now: Instant,
    ) {
        let mut packet = Vec::with_capacity(4 + body.len());
        packet.extend_from_slice(&MAGIC);
        packet.push(VERSION);
        packet.push(kind);
        packet.extend_from_slice(body);
        self.transmit(socket, addr, packet, now);
    }

    fn receive(&mut self, socket: &UdpSocket, buf: &[u8], from: SocketAddr, now: Instant) {
        if let Some((id, mapped)) = super::stun::parse_response(buf) {
            if let Some(index) = self.stun_ids.iter().position(|t| *t == id) {
                self.stun_ids.remove(index);
                let _ = self.events.send(LinkEvent::Mapped(mapped));
            }
            return;
        }
        if buf.len() < 4 || buf[0..2] != MAGIC || buf[2] != VERSION {
            return;
        }
        let body = &buf[4..];
        match buf[3] {
            KIND_CONNECT if body.len() >= 12 => {
                let room = u32::from_be_bytes(body[0..4].try_into().unwrap());
                let id = u64::from_be_bytes(body[4..12].try_into().unwrap());
                let Some(listening) = self.room else { return };
                if room != listening && room != 0 {
                    return;
                }
                if let Some(peer) = self.peers.get_mut(&id) {
                    peer.addr = from;
                    peer.last_heard = now;
                } else {
                    self.peers.insert(id, Peer::new(from, now));
                    let _ = self.events.send(LinkEvent::Connected {
                        peer: id,
                        addr: from,
                    });
                }
                // The guest is reachable now; stop punching towards it.
                self.punches.retain(|p| !p.candidates.contains(&from));
                self.control(socket, from, KIND_ACCEPT, &id.to_be_bytes(), now);
            }
            KIND_ACCEPT if body.len() >= 8 => {
                let id = u64::from_be_bytes(body[0..8].try_into().unwrap());
                if let Some(index) = self.attempts.iter().position(|a| a.peer == id) {
                    self.attempts.remove(index);
                    self.peers.insert(id, Peer::new(from, now));
                    let _ = self.events.send(LinkEvent::Connected {
                        peer: id,
                        addr: from,
                    });
                    // Anything queued before the answer goes now.
                    self.send_data(socket, id, now, true);
                }
            }
            KIND_PUNCH if body.len() >= 4 => {
                // The host is punching towards us: answer from here too.
                let room = u32::from_be_bytes(body[0..4].try_into().unwrap());
                let mut answers = Vec::new();
                for attempt in &mut self.attempts {
                    if attempt.room == room {
                        if !attempt.candidates.contains(&from) {
                            attempt.candidates.push(from);
                        }
                        answers.push((attempt.room, attempt.peer));
                    }
                }
                for (room, id) in answers {
                    let mut body = room.to_be_bytes().to_vec();
                    body.extend_from_slice(&id.to_be_bytes());
                    self.control(socket, from, KIND_CONNECT, &body, now);
                }
            }
            KIND_CLOSE if body.len() >= 8 => {
                let id = u64::from_be_bytes(body[0..8].try_into().unwrap());
                if self.peers.remove(&id).is_some() {
                    let _ = self.events.send(LinkEvent::Closed { peer: id });
                }
            }
            // The header alone (27 bytes) is a keepalive or a bare
            // acknowledgement: it too says the peer is there.
            KIND_DATA if body.len() >= DATA_HEADER => self.receive_data(socket, body, from, now),
            _ => {}
        }
    }

    fn receive_data(&mut self, socket: &UdpSocket, body: &[u8], from: SocketAddr, now: Instant) {
        let read_u32 = |at: usize| u32::from_be_bytes(body[at..at + 4].try_into().unwrap());
        let id = u64::from_be_bytes(body[0..8].try_into().unwrap());
        let now_stamp = self.stamp(now);
        let Some(peer) = self.peers.get_mut(&id) else {
            // A connection this side has forgotten: tell the sender once.
            self.control(socket, from, KIND_CLOSE, &id.to_be_bytes(), now);
            return;
        };
        // The connection id, not the address, names the peer.
        peer.addr = from;
        peer.last_heard = now;
        let stamp = read_u32(8);
        let echo = read_u32(12);
        let held = u16::from_be_bytes(body[16..18].try_into().unwrap());
        let ack = read_u32(18);
        let bits = read_u32(22);
        let count = body[26];
        peer.echo = Some((stamp, now));
        if echo != 0 {
            let rtt = f64::from(now_stamp.saturating_sub(echo)) - f64::from(held);
            if (0.0..10_000.0).contains(&rtt) {
                peer.sample_rtt(rtt.max(0.5));
            }
        }
        while peer.unacked.front().is_some_and(|s| s.seq < ack) {
            peer.unacked.pop_front();
        }
        if bits != 0 {
            peer.unacked.retain(|s| {
                let offset = s.seq.wrapping_sub(ack).wrapping_sub(1);
                !(offset < 32 && bits & (1 << offset) != 0)
            });
        }
        let mut at = DATA_HEADER;
        let mut messages = Vec::new();
        for _ in 0..count {
            if body.len() < at + 7 {
                break;
            }
            let seq = read_u32(at);
            let more = body[at + 4] != 0;
            let len = u16::from_be_bytes(body[at + 5..at + 7].try_into().unwrap()) as usize;
            at += 7;
            if body.len() < at + len {
                break;
            }
            let bytes = &body[at..at + len];
            at += len;
            peer.ack_owed.get_or_insert(now);
            if seq < peer.recv_next || seq >= peer.recv_next + 8192 {
                continue;
            }
            peer.recv_buf
                .entry(seq)
                .or_insert_with(|| (more, bytes.to_vec()));
        }
        while let Some((more, bytes)) = peer.recv_buf.remove(&peer.recv_next) {
            peer.recv_next += 1;
            peer.assembling.extend_from_slice(&bytes);
            if !more {
                messages.push(std::mem::take(&mut peer.assembling));
            }
        }
        for bytes in messages {
            let _ = self.events.send(LinkEvent::Message { peer: id, bytes });
        }
    }

    /// Everything the clock drives: resends, owed acknowledgements,
    /// keepalives, connection attempts, punches, forgetting the silent.
    fn timers(&mut self, socket: &UdpSocket, now: Instant) {
        self.flush_delayed(socket, now);
        let ids: Vec<PeerId> = self.peers.keys().copied().collect();
        for id in ids {
            let peer = &self.peers[&id];
            if now.duration_since(peer.last_heard) >= FORGET_AFTER {
                self.peers.remove(&id);
                let _ = self.events.send(LinkEvent::Closed { peer: id });
                continue;
            }
            let ack_due = peer
                .ack_owed
                .is_some_and(|since| now.duration_since(since) >= ACK_DELAY);
            let keepalive = now.duration_since(peer.last_sent) >= KEEPALIVE;
            if peer.wants_send(now) || ack_due || keepalive {
                // A full window drains over several packets.
                for _ in 0..8 {
                    if !self.send_data(socket, id, now, true) {
                        break;
                    }
                    if !self.peers[&id].wants_send(now) {
                        break;
                    }
                }
            }
        }
        let mut failed = Vec::new();
        let mut sends = Vec::new();
        self.attempts.retain_mut(|attempt| {
            if now >= attempt.until {
                failed.push(attempt.peer);
                return false;
            }
            if now >= attempt.next {
                attempt.next = now + RETRY_EVERY;
                let mut body = attempt.room.to_be_bytes().to_vec();
                body.extend_from_slice(&attempt.peer.to_be_bytes());
                for addr in &attempt.candidates {
                    sends.push((*addr, KIND_CONNECT, body.clone()));
                }
            }
            true
        });
        self.punches.retain_mut(|punch| {
            if now >= punch.until {
                return false;
            }
            if now >= punch.next {
                punch.next = now + RETRY_EVERY;
                for addr in &punch.candidates {
                    sends.push((*addr, KIND_PUNCH, punch.room.to_be_bytes().to_vec()));
                }
            }
            true
        });
        for (addr, kind, body) in sends {
            self.control(socket, addr, kind, &body, now);
        }
        for peer in failed {
            let _ = self.events.send(LinkEvent::ConnectFailed { peer });
        }
    }

    /// How long the thread may sleep before `timers` has work.
    fn idle_for(&self, now: Instant) -> Duration {
        let mut wait = Duration::from_millis(10);
        if !self.delayed.is_empty() {
            wait = Duration::from_millis(1);
        }
        for peer in self.peers.values() {
            if peer.ack_owed.is_some() || peer.wants_send(now) {
                wait = wait.min(Duration::from_millis(2));
            } else if let Some(front) = peer.unacked.front().and_then(|s| s.sent_at) {
                let due = (front + peer.rto()).saturating_duration_since(now);
                wait = wait.min(due.max(Duration::from_millis(1)));
            }
        }
        wait
    }
}

fn random_u64() -> u64 {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    hasher.write_u32(std::process::id());
    hasher.finish()
}

/// A nonzero random id: connection ids, rooms, rejoin tokens.
pub fn random_id() -> u64 {
    loop {
        let id = random_u64();
        if id != 0 {
            return id;
        }
    }
}

/// One UDP socket and every connection on it.
pub struct Endpoint {
    socket: UdpSocket,
    shared: Arc<Mutex<Shared>>,
    events: Receiver<LinkEvent>,
    thread: Option<JoinHandle<()>>,
    local: SocketAddr,
}

impl Endpoint {
    pub fn bind(addr: impl ToSocketAddrs, conditions: Conditions) -> Result<Endpoint, String> {
        let socket = UdpSocket::bind(addr).map_err(|e| format!("open the network port: {e}"))?;
        let local = socket.local_addr().map_err(|e| e.to_string())?;
        let (tx, events) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared {
            epoch: Instant::now(),
            room: None,
            peers: HashMap::new(),
            attempts: Vec::new(),
            punches: Vec::new(),
            conditions,
            rng: random_u64() | 1,
            delayed: Vec::new(),
            stun_ids: Vec::new(),
            events: tx,
            closing: false,
        }));
        let thread_socket = socket.try_clone().map_err(|e| e.to_string())?;
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("brinewake-net".into())
            .spawn(move || run(thread_socket, thread_shared))
            .map_err(|e| e.to_string())?;
        Ok(Endpoint {
            socket,
            shared,
            events,
            thread: Some(thread),
            local,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// Change the simulated network from now on: a test cuts a path with
    /// `loss_percent: 100` and restores it later.
    #[cfg(test)]
    pub fn set_conditions(&self, conditions: Conditions) {
        self.lock().conditions = conditions;
    }

    /// Accept connections for `room` (and for a bare address) from now on.
    pub fn listen(&self, room: u32) {
        self.lock().room = Some(room);
    }

    /// Try each candidate address until one answers or `wait` runs out.
    /// The answer is a `Connected` event carrying the returned id.
    pub fn connect(&self, candidates: &[SocketAddr], room: u32, wait: Duration) -> PeerId {
        let id = random_id();
        let now = Instant::now();
        self.lock().attempts.push(Attempt {
            peer: id,
            room,
            candidates: candidates.to_vec(),
            next: now,
            until: now + wait,
        });
        id
    }

    /// Send towards a guest that cannot reach this endpoint yet, so its
    /// router and ours both let the other through.
    pub fn punch(&self, candidates: &[SocketAddr], room: u32, wait: Duration) {
        let now = Instant::now();
        self.lock().punches.push(Punch {
            room,
            candidates: candidates.to_vec(),
            next: now,
            until: now + wait,
        });
    }

    /// Ask STUN servers which address this socket's packets arrive from.
    /// The answer is a `Mapped` event, or nothing if none replies.
    pub fn discover_mapping(&self, servers: &[&str]) {
        let Ok(socket) = self.socket.try_clone() else {
            return;
        };
        let shared = Arc::clone(&self.shared);
        let servers: Vec<String> = servers.iter().map(|s| s.to_string()).collect();
        let _ = std::thread::Builder::new()
            .name("brinewake-stun".into())
            .spawn(move || {
                for round in 0..3 {
                    for server in &servers {
                        let Ok(addrs) = server.to_socket_addrs() else {
                            continue;
                        };
                        for addr in addrs.filter(SocketAddr::is_ipv4).take(1) {
                            let id: [u8; 12] = {
                                let a = random_u64().to_be_bytes();
                                let b = random_u64().to_be_bytes();
                                let mut id = [0; 12];
                                id[..8].copy_from_slice(&a);
                                id[8..].copy_from_slice(&b[..4]);
                                id
                            };
                            {
                                let mut shared = shared.lock().unwrap_or_else(|e| e.into_inner());
                                if shared.closing {
                                    return;
                                }
                                shared.stun_ids.push(id);
                            }
                            let _ = socket.send_to(&super::stun::request(id), addr);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(400 * (round + 1)));
                }
            });
    }

    /// Queue one message for `peer` and send what can go now.
    pub fn send(&self, peer: PeerId, bytes: &[u8]) -> Result<(), String> {
        let mut shared = self.lock();
        let now = Instant::now();
        let Some(state) = shared.peers.get_mut(&peer) else {
            return Err("the other player is not connected".to_string());
        };
        let pieces = bytes.len().div_ceil(SEGMENT_BYTES).max(1);
        for index in 0..pieces {
            let start = index * SEGMENT_BYTES;
            let end = (start + SEGMENT_BYTES).min(bytes.len());
            let seq = state.next_seq;
            state.next_seq += 1;
            state.unacked.push_back(Segment {
                seq,
                more: index + 1 < pieces,
                bytes: bytes[start..end].to_vec(),
                sent_at: None,
                sends: 0,
            });
        }
        shared.send_data(&self.socket, peer, now, false);
        Ok(())
    }

    /// The next event, if any.
    pub fn poll(&self) -> Option<LinkEvent> {
        self.events.try_recv().ok()
    }

    /// Wait up to `timeout` for the next event.
    #[cfg(test)]
    pub fn wait(&self, timeout: Duration) -> Option<LinkEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    pub fn stats(&self, peer: PeerId) -> Option<PeerStats> {
        let shared = self.lock();
        let now = Instant::now();
        shared.peers.get(&peer).map(|p| PeerStats {
            addr: p.addr,
            rtt_ms: p.srtt_ms.map(|ms| ms.round() as u32),
            jitter_ms: p.rttvar_ms.round() as u32,
            quiet: now.duration_since(p.last_heard),
            packets_sent: p.packets_sent,
            segments_resent: p.segments_resent,
        })
    }

    /// Say goodbye to one peer and forget it.
    pub fn disconnect(&self, peer: PeerId) {
        let mut shared = self.lock();
        let now = Instant::now();
        // Flush what is queued (a final batch, a goodbye message) first.
        shared.send_data(&self.socket, peer, now, false);
        if let Some(state) = shared.peers.remove(&peer) {
            for _ in 0..3 {
                shared.control(
                    &self.socket,
                    state.addr,
                    KIND_CLOSE,
                    &peer.to_be_bytes(),
                    now,
                );
            }
        }
        shared.attempts.retain(|a| a.peer != peer);
    }

    /// Wait up to `timeout` for every peer to acknowledge what was sent.
    pub fn flush(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let pending = {
                let shared = self.lock();
                shared.peers.values().any(|p| !p.unacked.is_empty()) || !shared.delayed.is_empty()
            };
            if !pending {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        let peers: Vec<PeerId> = self.lock().peers.keys().copied().collect();
        for peer in peers {
            self.disconnect(peer);
        }
        {
            let mut shared = self.lock();
            // The simulated network still owes these to the wire.
            let now = Instant::now() + Duration::from_secs(3600);
            shared.flush_delayed(&self.socket, now);
            shared.closing = true;
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(socket: UdpSocket, shared: Arc<Mutex<Shared>>) {
    let mut buf = vec![0u8; 2048];
    let mut wait = Duration::from_millis(10);
    loop {
        let _ = socket.set_read_timeout(Some(wait));
        let received = socket.recv_from(&mut buf);
        let now = Instant::now();
        let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
        if state.closing {
            return;
        }
        if let Ok((n, from)) = received {
            state.receive(&socket, &buf[..n], from, now);
        }
        state.timers(&socket, now);
        wait = state.idle_for(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: Conditions, b: Conditions) -> (Endpoint, PeerId, Endpoint, PeerId) {
        let host = Endpoint::bind("127.0.0.1:0", a).expect("host binds");
        host.listen(42);
        let guest = Endpoint::bind("127.0.0.1:0", b).expect("guest binds");
        let id = guest.connect(&[host.local_addr()], 42, Duration::from_secs(5));
        let deadline = Instant::now() + Duration::from_secs(5);
        let (mut host_peer, mut guest_peer) = (None, None);
        while (host_peer.is_none() || guest_peer.is_none()) && Instant::now() < deadline {
            if let Some(LinkEvent::Connected { peer, .. }) = host.wait(Duration::from_millis(5)) {
                host_peer = Some(peer);
            }
            if let Some(LinkEvent::Connected { peer, .. }) = guest.wait(Duration::from_millis(5)) {
                guest_peer = Some(peer);
            }
        }
        assert_eq!(guest_peer, Some(id));
        assert_eq!(host_peer, Some(id), "both ends name the connection alike");
        (host, id, guest, id)
    }

    fn collect(endpoint: &Endpoint, want: usize, within: Duration) -> Vec<Vec<u8>> {
        let deadline = Instant::now() + within;
        let mut got = Vec::new();
        while got.len() < want && Instant::now() < deadline {
            if let Some(LinkEvent::Message { bytes, .. }) = endpoint.wait(Duration::from_millis(5))
            {
                got.push(bytes);
            }
        }
        got
    }

    #[test]
    fn conditions_parse_and_refuse_nonsense() {
        let c = Conditions::parse("latency=80, jitter=30ms,loss=10%").unwrap();
        assert_eq!(
            c,
            Conditions {
                latency_ms: 80,
                jitter_ms: 30,
                loss_percent: 10
            }
        );
        assert!(Conditions::parse("speed=3").is_err());
        assert!(Conditions::parse("loss=lots").is_err());
        assert_eq!(Conditions::parse("").unwrap(), Conditions::default());
    }

    #[test]
    fn messages_arrive_whole_and_in_order_over_a_clean_path() {
        let (host, hp, guest, gp) = pair(Conditions::default(), Conditions::default());
        for i in 0..50u32 {
            guest.send(gp, &i.to_be_bytes()).unwrap();
        }
        let big: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
        guest.send(gp, &big).unwrap();
        let got = collect(&host, 51, Duration::from_secs(5));
        assert_eq!(got.len(), 51);
        for (i, bytes) in got.iter().take(50).enumerate() {
            assert_eq!(bytes.as_slice(), &(i as u32).to_be_bytes());
        }
        assert_eq!(
            got[50], big,
            "a message larger than a packet is reassembled"
        );
        host.send(hp, b"back").unwrap();
        assert_eq!(
            collect(&guest, 1, Duration::from_secs(2)),
            vec![b"back".to_vec()]
        );
    }

    #[test]
    fn a_lossy_jittery_path_still_delivers_everything_in_order() {
        let bad = Conditions {
            latency_ms: 40,
            jitter_ms: 30,
            loss_percent: 20,
        };
        let (host, hp, guest, gp) = pair(bad, bad);
        let started = Instant::now();
        let mut expected = Vec::new();
        for i in 0..300u32 {
            let bytes = format!("batch {i}").into_bytes();
            guest.send(gp, &bytes).unwrap();
            expected.push(bytes);
            if i % 10 == 0 {
                host.send(hp, &i.to_be_bytes()).unwrap();
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let got = collect(&host, 300, Duration::from_secs(20));
        assert_eq!(got, expected, "every message, once, in order");
        let back = collect(&guest, 30, Duration::from_secs(10));
        assert_eq!(back.len(), 30);
        let stats = guest.stats(gp).unwrap();
        let rtt = stats.rtt_ms.expect("measured");
        assert!(
            (70..250).contains(&rtt),
            "round trip near 2 x (40 + jitter): {rtt}"
        );
        assert!(stats.segments_resent > 0, "losses were repaired");
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[test]
    fn a_peer_whose_address_changes_carries_on() {
        let (host, hp, guest, gp) = pair(Conditions::default(), Conditions::default());
        guest.send(gp, b"one").unwrap();
        assert_eq!(collect(&host, 1, Duration::from_secs(2)).len(), 1);
        // The old address goes quiet, as it would once the router moved on.
        guest.set_conditions(Conditions {
            loss_percent: 100,
            ..Conditions::default()
        });
        // A second socket speaking for the same connection, as a router
        // rebinding the guest's port would look to the host.
        let moved = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut packet = MAGIC.to_vec();
        packet.extend_from_slice(&[VERSION, KIND_DATA]);
        packet.extend_from_slice(&gp.to_be_bytes());
        packet.extend_from_slice(&5u32.to_be_bytes()); // stamp
        packet.extend_from_slice(&0u32.to_be_bytes()); // echo
        packet.extend_from_slice(&0u16.to_be_bytes()); // held
        packet.extend_from_slice(&0u32.to_be_bytes()); // ack
        packet.extend_from_slice(&0u32.to_be_bytes()); // bits
        packet.push(1);
        packet.extend_from_slice(&1u32.to_be_bytes()); // seq 1 follows "one"
        packet.push(0);
        packet.extend_from_slice(&3u16.to_be_bytes());
        packet.extend_from_slice(b"two");
        moved.send_to(&packet, host.local_addr()).unwrap();
        assert_eq!(
            collect(&host, 1, Duration::from_secs(2)),
            vec![b"two".to_vec()]
        );
        assert_eq!(
            host.stats(hp).unwrap().addr,
            moved.local_addr().unwrap(),
            "replies go to where the peer is now"
        );
    }

    #[test]
    fn a_wrong_room_is_not_answered_and_the_attempt_gives_up() {
        let host = Endpoint::bind("127.0.0.1:0", Conditions::default()).unwrap();
        host.listen(7);
        let guest = Endpoint::bind("127.0.0.1:0", Conditions::default()).unwrap();
        let id = guest.connect(&[host.local_addr()], 8, Duration::from_millis(400));
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut failed = false;
        while Instant::now() < deadline && !failed {
            if let Some(LinkEvent::ConnectFailed { peer }) = guest.wait(Duration::from_millis(10)) {
                failed = peer == id;
            }
        }
        assert!(failed);
        assert!(host.poll().is_none(), "the host heard nothing it accepted");
    }

    #[test]
    fn a_punch_from_the_host_opens_the_way_for_a_guest() {
        // The guest tries an address where nothing listens (the host's
        // public address as a strict router would treat it), then the host
        // punches towards the guest, which answers the punch directly.
        let host = Endpoint::bind("127.0.0.1:0", Conditions::default()).unwrap();
        host.listen(99);
        let guest = Endpoint::bind("127.0.0.1:0", Conditions::default()).unwrap();
        let dead = UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let id = guest.connect(&[dead], 99, Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(250));
        host.punch(&[guest.local_addr()], 99, Duration::from_secs(5));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut connected = false;
        while Instant::now() < deadline && !connected {
            if let Some(LinkEvent::Connected { peer, .. }) = guest.wait(Duration::from_millis(10)) {
                connected = peer == id;
            }
        }
        assert!(connected);
    }

    #[test]
    fn a_quiet_peer_is_still_heard() {
        // A seat with nothing to send (a player still reading, a lockstep
        // seat that has not started stepping) sends only keepalives, and
        // those must keep it alive at the other end.
        let (host, hp, guest, _gp) = pair(Conditions::default(), Conditions::default());
        let deadline = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < deadline {
            while guest.poll().is_some() {}
            while host.poll().is_some() {}
            std::thread::sleep(Duration::from_millis(20));
        }
        let quiet = host.stats(hp).expect("the guest is known").quiet;
        assert!(quiet < Duration::from_millis(600), "heard {quiet:?} ago");
    }

    #[test]
    fn a_host_resends_nothing_a_quiet_guest_has_acknowledged() {
        let (host, hp, guest, _gp) = pair(Conditions::default(), Conditions::default());
        host.send(hp, b"one message").unwrap();
        assert_eq!(collect(&guest, 1, Duration::from_secs(2)).len(), 1);
        std::thread::sleep(Duration::from_millis(1500));
        let stats = host.stats(hp).expect("the guest is known");
        assert_eq!(stats.segments_resent, 0, "the bare acknowledgement arrived");
    }

    #[test]
    fn a_goodbye_is_heard() {
        let (host, _hp, guest, gp) = pair(Conditions::default(), Conditions::default());
        guest.send(gp, b"last words").unwrap();
        guest.disconnect(gp);
        let deadline = Instant::now() + Duration::from_secs(3);
        let (mut words, mut closed) = (false, false);
        while Instant::now() < deadline && !closed {
            match host.wait(Duration::from_millis(10)) {
                Some(LinkEvent::Message { bytes, .. }) => words = bytes == b"last words",
                Some(LinkEvent::Closed { .. }) => closed = true,
                _ => {}
            }
        }
        assert!(words, "what was queued before the goodbye arrives first");
        assert!(closed);
    }
}
