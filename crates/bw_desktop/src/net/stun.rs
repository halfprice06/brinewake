//! The two halves of a STUN binding request (RFC 5389): enough to learn the
//! public address a home router gives this game's socket.
//!
//! Brinewake runs no servers. A public STUN server only answers "your
//! packets came from 203.0.113.7:51234"; no game traffic goes near it. When
//! none answers, the join code carries the local network address alone.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

const COOKIE: u32 = 0x2112_A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const MAPPED_ADDRESS: u16 = 0x0001;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// Public servers asked in turn. Any one answering is enough.
pub const SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "stun1.l.google.com:19302",
];

pub fn request(id: [u8; 12]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(20);
    packet.extend_from_slice(&BINDING_REQUEST.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&COOKIE.to_be_bytes());
    packet.extend_from_slice(&id);
    packet
}

/// The transaction id and mapped address of a binding success, or `None`
/// for anything else (including every Brinewake packet).
pub fn parse_response(buf: &[u8]) -> Option<([u8; 12], SocketAddr)> {
    if buf.len() < 20 || buf[0] & 0xC0 != 0 {
        return None;
    }
    if u16::from_be_bytes([buf[0], buf[1]]) != BINDING_SUCCESS
        || u32::from_be_bytes(buf[4..8].try_into().ok()?) != COOKIE
    {
        return None;
    }
    let length = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    let id: [u8; 12] = buf[8..20].try_into().ok()?;
    let body = buf.get(20..20 + length)?;
    let mut at = 0;
    let mut plain = None;
    while at + 4 <= body.len() {
        let kind = u16::from_be_bytes([body[at], body[at + 1]]);
        let len = u16::from_be_bytes([body[at + 2], body[at + 3]]) as usize;
        let value = body.get(at + 4..at + 4 + len)?;
        match kind {
            XOR_MAPPED_ADDRESS => return Some((id, decode(value, true, &id)?)),
            MAPPED_ADDRESS => plain = decode(value, false, &id),
            _ => {}
        }
        // Attributes are padded to four bytes.
        at += 4 + len.div_ceil(4) * 4;
    }
    plain.map(|addr| (id, addr))
}

fn decode(value: &[u8], xor: bool, id: &[u8; 12]) -> Option<SocketAddr> {
    if value.len() < 4 {
        return None;
    }
    let family = value[1];
    let mut port = u16::from_be_bytes([value[2], value[3]]);
    if xor {
        port ^= (COOKIE >> 16) as u16;
    }
    let mut mask = COOKIE.to_be_bytes().to_vec();
    mask.extend_from_slice(id);
    match family {
        0x01 if value.len() >= 8 => {
            let mut octets: [u8; 4] = value[4..8].try_into().ok()?;
            if xor {
                for (o, m) in octets.iter_mut().zip(&mask) {
                    *o ^= m;
                }
            }
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(octets)), port))
        }
        0x02 if value.len() >= 20 => {
            let mut octets: [u8; 16] = value[4..20].try_into().ok()?;
            if xor {
                for (o, m) in octets.iter_mut().zip(&mask) {
                    *o ^= m;
                }
            }
            Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binding_success_names_the_public_address() {
        let id = [7u8; 12];
        let public: SocketAddr = "203.0.113.7:51234".parse().unwrap();
        // Build the answer a server would send.
        let mut value = vec![0, 1];
        value.extend_from_slice(&(51234u16 ^ (COOKIE >> 16) as u16).to_be_bytes());
        let cookie = COOKIE.to_be_bytes();
        for (i, octet) in [203u8, 0, 113, 7].iter().enumerate() {
            value.push(octet ^ cookie[i]);
        }
        let mut attrs = Vec::new();
        // An unknown, padded attribute first.
        attrs.extend_from_slice(&0x8022u16.to_be_bytes());
        attrs.extend_from_slice(&3u16.to_be_bytes());
        attrs.extend_from_slice(b"abc\0");
        attrs.extend_from_slice(&XOR_MAPPED_ADDRESS.to_be_bytes());
        attrs.extend_from_slice(&(value.len() as u16).to_be_bytes());
        attrs.extend_from_slice(&value);
        let mut packet = BINDING_SUCCESS.to_be_bytes().to_vec();
        packet.extend_from_slice(&(attrs.len() as u16).to_be_bytes());
        packet.extend_from_slice(&cookie);
        packet.extend_from_slice(&id);
        packet.extend_from_slice(&attrs);
        assert_eq!(parse_response(&packet), Some((id, public)));
    }

    #[test]
    fn a_request_is_a_well_formed_binding_request_and_game_packets_are_not_stun() {
        let packet = request([1; 12]);
        assert_eq!(packet.len(), 20);
        assert_eq!(&packet[0..2], &[0, 1]);
        assert_eq!(&packet[4..8], &COOKIE.to_be_bytes());
        assert_eq!(parse_response(&packet), None, "a request is not an answer");
        assert_eq!(parse_response(b"BW\x01\x03 and some more bytes here"), None);
    }
}
