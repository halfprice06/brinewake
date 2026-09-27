//! Join codes: where a host can be reached, in a form one player can paste
//! to another in any chat.
//!
//! A code carries a random room number, so a stray connection never lands
//! in someone else's match, and up to three addresses: the public one a
//! STUN server saw, the router's own mapping if UPnP made one, and the
//! local network address for players in the same house. It is written in
//! Crockford base32 (no I, L, O or U; case and dashes do not matter) with
//! a checksum, so a mistyped code is refused rather than tried.
//!
//! A reply code is the same thing in the other direction: a guest whose
//! packets cannot reach the host sends one back, and the host sends
//! towards those addresses so both routers open.
//!
//! A bare `address:port` (or an address alone, on the default port) is
//! accepted wherever a code is.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};

/// The port a host listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 47800;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const HOST_CODE: u8 = 1;
const REPLY_CODE: u8 = 2;
const MAX_ADDRESSES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    /// Made by a host: join with it.
    Host,
    /// Made by a guest the host could not hear: the host adds it.
    Reply,
    /// Typed as an address.
    Address,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinCode {
    pub kind: CodeKind,
    /// Zero for a typed address, which any host accepts.
    pub room: u32,
    pub addresses: Vec<SocketAddr>,
}

fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn unbase32(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for c in text.chars() {
        let c = match c.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            c => c,
        };
        let value = ALPHABET.iter().position(|a| *a as char == c)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

impl JoinCode {
    fn encode(&self, version: u8) -> String {
        let mut bytes = vec![version];
        bytes.extend_from_slice(&self.room.to_be_bytes());
        let mut seen = Vec::new();
        for addr in &self.addresses {
            if let SocketAddr::V4(v4) = addr
                && !seen.contains(v4)
                && seen.len() < MAX_ADDRESSES
            {
                seen.push(*v4);
                bytes.extend_from_slice(&v4.ip().octets());
                bytes.extend_from_slice(&v4.port().to_be_bytes());
            }
        }
        bytes.push(crc8(&bytes));
        let body = base32(&bytes);
        let groups: Vec<&str> = body
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();
        format!("BW-{}", groups.join("-"))
    }

    /// The code a host shows.
    pub fn host(room: u32, addresses: &[SocketAddr]) -> String {
        JoinCode {
            kind: CodeKind::Host,
            room,
            addresses: addresses.to_vec(),
        }
        .encode(HOST_CODE)
    }

    /// The code a guest sends back when it cannot reach the host.
    pub fn reply(room: u32, addresses: &[SocketAddr]) -> String {
        JoinCode {
            kind: CodeKind::Reply,
            room,
            addresses: addresses.to_vec(),
        }
        .encode(REPLY_CODE)
    }

    /// Read a code, or an address typed instead of one.
    pub fn parse(text: &str) -> Result<JoinCode, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("Paste a join code or type an address.".into());
        }
        let compact: String = text
            .chars()
            .filter(|c| !matches!(c, '-' | ' ' | '\t'))
            .collect();
        let looks_like_code = compact.len() > 2
            && compact[..2].eq_ignore_ascii_case("BW")
            && !compact.contains(['.', ':']);
        if looks_like_code {
            let bytes = unbase32(&compact[2..]).ok_or("That code has a letter codes never use.")?;
            // Trailing bits of the last character pad to a byte boundary.
            let bytes = trim_to_payload(&bytes).ok_or("That code is incomplete or mistyped.")?;
            let kind = match bytes[0] {
                HOST_CODE => CodeKind::Host,
                REPLY_CODE => CodeKind::Reply,
                _ => return Err("That code is from a different version of Brinewake.".into()),
            };
            let room = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
            let addresses = bytes[5..bytes.len() - 1]
                .chunks(6)
                .map(|c| {
                    SocketAddr::V4(SocketAddrV4::new(
                        Ipv4Addr::new(c[0], c[1], c[2], c[3]),
                        u16::from_be_bytes([c[4], c[5]]),
                    ))
                })
                .collect();
            return Ok(JoinCode {
                kind,
                room,
                addresses,
            });
        }
        let with_port = if text.parse::<IpAddr>().is_ok() || !text.contains(':') {
            match text.parse::<IpAddr>() {
                Ok(IpAddr::V6(v6)) => format!("[{v6}]:{DEFAULT_PORT}"),
                _ => format!("{text}:{DEFAULT_PORT}"),
            }
        } else {
            text.to_string()
        };
        let addresses: Vec<SocketAddr> = with_port
            .to_socket_addrs()
            .map_err(|_| format!("Couldn't find {text}. Check the code or address."))?
            .collect();
        if addresses.is_empty() {
            return Err(format!("Couldn't find {text}."));
        }
        Ok(JoinCode {
            kind: CodeKind::Address,
            room: 0,
            addresses,
        })
    }
}

/// The payload is 5 bytes, whole addresses and a checksum; base32 may add a
/// padding byte of zero bits at the end.
fn trim_to_payload(bytes: &[u8]) -> Option<&[u8]> {
    for len in [bytes.len(), bytes.len().saturating_sub(1)] {
        if len >= 6 && (len - 6) % 6 == 0 && (len - 6) / 6 <= MAX_ADDRESSES {
            let payload = &bytes[..len];
            if crc8(&payload[..len - 1]) == payload[len - 1] {
                return Some(payload);
            }
        }
    }
    None
}

/// This machine's address on its local network: the one a packet to the
/// internet would leave from. Nothing is sent to find it.
pub fn local_network_address() -> Option<Ipv4Addr> {
    let probe = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    probe.connect("192.0.2.1:9").ok()?;
    match probe.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_unspecified() && !ip.is_loopback() => Some(ip),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(text: &str) -> SocketAddr {
        text.parse().unwrap()
    }

    #[test]
    fn a_host_code_round_trips_whatever_its_spelling() {
        let addresses = [addr("203.0.113.7:51234"), addr("192.168.1.20:47800")];
        let code = JoinCode::host(0xDEAD_BEEF, &addresses);
        assert!(code.starts_with("BW-"));
        assert!(code.len() < 40, "short enough to read out: {code}");
        let parsed = JoinCode::parse(&code).unwrap();
        assert_eq!(parsed.kind, CodeKind::Host);
        assert_eq!(parsed.room, 0xDEAD_BEEF);
        assert_eq!(parsed.addresses, addresses);
        // Lower case, no dashes, spaces around, O for 0 and L for 1.
        let sloppy = format!("  {}  ", code.to_lowercase().replace('-', ""))
            .replace('0', "o")
            .replace('1', "l");
        assert_eq!(JoinCode::parse(&sloppy).unwrap(), parsed);
    }

    #[test]
    fn a_mistyped_code_is_refused_not_tried() {
        let code = JoinCode::host(77, &[addr("198.51.100.4:47800")]);
        let mut chars: Vec<char> = code.chars().collect();
        let last = chars.len() - 3;
        chars[last] = if chars[last] == '7' { '8' } else { '7' };
        let typo: String = chars.into_iter().collect();
        assert!(JoinCode::parse(&typo).is_err());
        assert!(JoinCode::parse(&code[..code.len() - 5]).is_err());
        assert!(JoinCode::parse("BW-UUUU").is_err());
    }

    #[test]
    fn a_reply_code_says_so_and_addresses_are_codes_too() {
        let reply = JoinCode::reply(5, &[addr("203.0.113.9:6000")]);
        assert_eq!(JoinCode::parse(&reply).unwrap().kind, CodeKind::Reply);
        let typed = JoinCode::parse("127.0.0.1:4790").unwrap();
        assert_eq!(typed.kind, CodeKind::Address);
        assert_eq!(typed.room, 0);
        assert_eq!(typed.addresses, vec![addr("127.0.0.1:4790")]);
        let bare = JoinCode::parse("10.0.0.5").unwrap();
        assert_eq!(bare.addresses, vec![addr("10.0.0.5:47800")]);
        assert!(JoinCode::parse("").is_err());
    }

    #[test]
    fn no_more_than_three_addresses_and_no_repeats() {
        let a = addr("1.2.3.4:5");
        let code = JoinCode::host(
            1,
            &[
                a,
                a,
                addr("5.6.7.8:9"),
                addr("9.9.9.9:1"),
                addr("8.8.8.8:2"),
            ],
        );
        let parsed = JoinCode::parse(&code).unwrap();
        assert_eq!(
            parsed.addresses,
            vec![a, addr("5.6.7.8:9"), addr("9.9.9.9:1")]
        );
    }
}
