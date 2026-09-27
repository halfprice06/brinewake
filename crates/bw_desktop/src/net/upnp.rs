//! Ask the home router to forward the host's game port (UPnP IGD).
//!
//! Most home routers accept this, and when one does, a guest anywhere can
//! reach the host with no port forwarding set up by hand. The request runs
//! on its own thread: finding the router can take a few seconds and must
//! never hold up the lobby. The mapping is leased for two hours and
//! removed when the host is done.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

const LEASE_SECONDS: u32 = 2 * 60 * 60;
const DESCRIPTION: &str = "Brinewake";

/// What the router said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Forwarded: guests can reach `external` directly.
    Mapped { external: SocketAddr },
    /// No router answered, or it refused. The reason is for the log.
    Unavailable(String),
}

/// A mapping request in flight, then its answer and the means to undo it.
pub struct PortMapping {
    rx: Receiver<(Outcome, Option<igd_next::Gateway>)>,
    outcome: Option<Outcome>,
    gateway: Option<igd_next::Gateway>,
}

/// The address of this machine on the router's network.
fn address_towards(gateway: SocketAddr) -> Option<IpAddr> {
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    probe.connect(gateway).ok()?;
    Some(probe.local_addr().ok()?.ip())
}

fn request(port: u16) -> (Outcome, Option<igd_next::Gateway>) {
    use igd_next::{PortMappingProtocol::UDP, SearchOptions, search_gateway};
    let options = SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        single_search_timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };
    let gateway = match search_gateway(options) {
        Ok(gateway) => gateway,
        Err(e) => return (Outcome::Unavailable(format!("no UPnP router: {e}")), None),
    };
    let Some(ip) = address_towards(gateway.addr) else {
        return (
            Outcome::Unavailable("no route to the router".into()),
            Some(gateway),
        );
    };
    let local = SocketAddr::new(ip, port);
    let external_ip = match gateway.get_external_ip() {
        Ok(ip) => ip,
        Err(e) => {
            return (
                Outcome::Unavailable(format!("router has no public address: {e}")),
                Some(gateway),
            );
        }
    };
    // The same port outside as in, if the router allows it: a code made
    // before the answer then still points the right way.
    let same = gateway
        .add_port(UDP, port, local, LEASE_SECONDS, DESCRIPTION)
        .or_else(|_| gateway.add_port(UDP, port, local, 0, DESCRIPTION));
    let external_port = match same {
        Ok(()) => Ok(port),
        Err(_) => gateway
            .add_any_port(UDP, local, LEASE_SECONDS, DESCRIPTION)
            .map_err(|e| e.to_string()),
    };
    match external_port {
        Ok(external_port) => (
            Outcome::Mapped {
                external: SocketAddr::new(external_ip, external_port),
            },
            Some(gateway),
        ),
        Err(e) => (
            Outcome::Unavailable(format!("router refused the mapping: {e}")),
            Some(gateway),
        ),
    }
}

impl PortMapping {
    /// Start asking the router to forward UDP `port` to this machine.
    pub fn request(port: u16) -> PortMapping {
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("brinewake-upnp".into())
            .spawn(move || {
                let _ = tx.send(request(port));
            });
        PortMapping {
            rx,
            outcome: None,
            gateway: None,
        }
    }

    /// The answer once it has come, without waiting.
    pub fn outcome(&mut self) -> Option<&Outcome> {
        if self.outcome.is_none()
            && let Ok((outcome, gateway)) = self.rx.try_recv()
        {
            self.outcome = Some(outcome);
            self.gateway = gateway;
        }
        self.outcome.as_ref()
    }
}

impl Drop for PortMapping {
    fn drop(&mut self) {
        let _ = self.outcome();
        if let (Some(Outcome::Mapped { external }), Some(gateway)) =
            (self.outcome.take(), self.gateway.take())
        {
            // Off the game thread: a router that has gone away must not
            // hold up leaving the lobby. The lease ends it anyway.
            let _ = std::thread::Builder::new()
                .name("brinewake-upnp-release".into())
                .spawn(move || {
                    let _ =
                        gateway.remove_port(igd_next::PortMappingProtocol::UDP, external.port());
                });
        }
    }
}
