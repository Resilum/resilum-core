//! HMAC-authenticated wire datagram: `header || payload || tag`.
//! Header packs a session id, seq, ack and kind.

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const HEADER_LEN: usize = 4 + 4 + 4 + 1; // session:u32, seq:u32, ack:u32, kind:u8
pub const DEFAULT_TAG_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Data = 1,
    Poll = 2,
    Handshake = 3,
}

impl Kind {
    pub fn from_u8(k: u8) -> Option<Self> {
        match k {
            1 => Some(Self::Data),
            2 => Some(Self::Poll),
            3 => Some(Self::Handshake),
            _ => None,
        }
    }
}

pub const fn overhead(tag_len: usize) -> usize {
    HEADER_LEN + tag_len
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datagram {
    pub session: u32,
    pub seq: u32,
    pub ack: u32,
    pub kind: Kind,
    pub payload: Vec<u8>,
}

/// Just enough of the header for demuxing without unpacking the payload.
pub fn peek_header(raw: &[u8]) -> Option<(u32, u32, u32, u8)> {
    if raw.len() < HEADER_LEN {
        return None;
    }
    let session = u32::from_be_bytes(raw[0..4].try_into().ok()?);
    let seq = u32::from_be_bytes(raw[4..8].try_into().ok()?);
    let ack = u32::from_be_bytes(raw[8..12].try_into().ok()?);
    let kind = raw[12];
    Some((session, seq, ack, kind))
}

fn write_header(dg: &Datagram) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + dg.payload.len());
    out.extend_from_slice(&dg.session.to_be_bytes());
    out.extend_from_slice(&dg.seq.to_be_bytes());
    out.extend_from_slice(&dg.ack.to_be_bytes());
    out.push(dg.kind as u8);
    out.extend_from_slice(&dg.payload);
    out
}

pub fn pack(dg: &Datagram, key: &[u8], tag_len: usize) -> Vec<u8> {
    let mut body = write_header(dg);
    let tag = hmac_tag(key, &body, tag_len);
    body.extend_from_slice(&tag);
    body
}

pub fn unpack(raw: &[u8], key: &[u8], tag_len: usize) -> Option<Datagram> {
    if raw.len() < HEADER_LEN + tag_len {
        return None;
    }
    let split = raw.len() - tag_len;
    let (body, tag) = raw.split_at(split);
    if tag_len > 0 {
        let expect = hmac_tag(key, body, tag_len);
        if !ct_eq(tag, &expect) {
            return None;
        }
    }
    let (session, seq, ack, kind_byte) = peek_header(body)?;
    let kind = Kind::from_u8(kind_byte)?;
    Some(Datagram {
        session,
        seq,
        ack,
        kind,
        payload: body[HEADER_LEN..].to_vec(),
    })
}

fn hmac_tag(key: &[u8], body: &[u8], tag_len: usize) -> Vec<u8> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC key of any length");
    mac.update(body);
    let full = mac.finalize().into_bytes();
    full[..tag_len].to_vec()
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests;
