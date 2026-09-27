//! Peer-to-peer matches: no servers, one player hosts.
//!
//! - `link`: reliable, ordered messages over one UDP socket, with loss
//!   repair, round-trip measurement and hole punching.
//! - `code`: join codes that carry a host's addresses.
//! - `stun`, `upnp`: learning the public address and asking the router to
//!   forward the port, so a guest anywhere can reach the host.
//! - `protocol`: the messages, and the match plan every seat builds from.
//! - `lobby`: finding each other, picking sides, starting together.
//! - `session`: the lockstep match itself.
//!
//! The design note is `docs/NETWORKING.md`.

pub mod code;
pub mod link;
pub mod lobby;
pub mod protocol;
pub mod session;
mod stun;
mod upnp;

#[cfg(test)]
pub(crate) mod tests;

pub use code::JoinCode;
pub use link::Conditions;
pub use lobby::{Lobby, LobbyOptions, Phase};
pub use session::{Ending, Session, StepOutcome};

/// A host and a guest on this machine, connected and started: for tests
/// that need a live session without the lobby.
#[cfg(test)]
pub fn local_pair(seed: u64, host_faction: bw_core::Faction, delay: u64) -> (Session, Session) {
    use std::time::Duration;
    let options = LobbyOptions {
        fixed_delay: Some(delay),
        auto: true,
        ..LobbyOptions::local()
    };
    let mut host = Lobby::host("127.0.0.1:0", host_faction, seed, options.clone()).expect("host");
    let addr = host.local_addr().expect("bound");
    let code = JoinCode::parse(&addr.to_string()).expect("address");
    let mut guest = Lobby::join(&code, None, options).expect("guest");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let (mut a, mut b) = (None, None);
    while (a.is_none() || b.is_none()) && std::time::Instant::now() < deadline {
        if a.is_none() {
            a = host.poll();
        }
        if b.is_none() {
            b = guest.poll();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let (mut a, mut b) = (a.expect("host started"), b.expect("guest started"));
    // Tests step at once rather than waiting for the shared start moment.
    a.start_now();
    b.start_now();
    (a, b)
}
