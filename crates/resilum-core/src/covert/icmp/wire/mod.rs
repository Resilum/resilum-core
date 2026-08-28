//! Build and parse ICMP/ICMPv6 echo messages carrying the wire datagram behind
//! a per-server marker. Building emits the ICMP body alone; the kernel adds the
//! IP header on send (and fills the ICMPv6 checksum).
//!
//! A datagram ping socket's id is kernel-assigned, so the tunnel is recognised
//! by the marker at the start of the payload rather than by the id. The id is
//! still what routes the kernel's delivery of a reply to the client socket, so
//! a reply echoes the id its request arrived with.

use std::net::IpAddr;

use super::marker::MARKER_LEN;

pub(super) const ETH_P_IP: u16 = 0x0800;
pub(super) const ETH_P_IPV6: u16 = 0x86DD;
const PROTO_ICMP: u8 = 1;
const PROTO_ICMPV6: u8 = 58;
const REQUEST_V4: u8 = 8;
const REPLY_V4: u8 = 0;
const REQUEST_V6: u8 = 128;
const REPLY_V6: u8 = 129;
const HEADER_LEN: usize = 8; // type + code + checksum + id + seq

fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for c in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([c[0], c[1]]) as u32);
    }
    if let &[last] = chunks.remainder() {
        sum = sum.wrapping_add(u16::from_be_bytes([last, 0]) as u32);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn echo(icmp_type: u8, id: u16, marker: [u8; MARKER_LEN], payload: &[u8], v6: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_LEN + MARKER_LEN + payload.len());
    buf.push(icmp_type);
    buf.push(0); // code
    buf.extend_from_slice(&[0, 0]); // checksum placeholder
    buf.extend_from_slice(&id.to_be_bytes());
    buf.extend_from_slice(&[0, 0]); // seq
    buf.extend_from_slice(&marker);
    buf.extend_from_slice(payload);
    if !v6 {
        // Kernel fills the ICMPv6 checksum; only IPv4 echoes need it here.
        let ck = checksum(&buf).to_be_bytes();
        buf[2] = ck[0];
        buf[3] = ck[1];
    }
    buf
}

const KERNEL_ASSIGNS_THE_ID: u16 = 0;

pub fn build_echo_request(marker: [u8; MARKER_LEN], payload: &[u8], v6: bool) -> Vec<u8> {
    let ty = if v6 { REQUEST_V6 } else { REQUEST_V4 };
    echo(ty, KERNEL_ASSIGNS_THE_ID, marker, payload, v6)
}

pub fn build_echo_reply(id: u16, marker: [u8; MARKER_LEN], payload: &[u8], v6: bool) -> Vec<u8> {
    let ty = if v6 { REPLY_V6 } else { REPLY_V4 };
    echo(ty, id, marker, payload, v6)
}

pub fn payload_of_reply(icmp: &[u8], marker: [u8; MARKER_LEN], v6: bool) -> Option<&[u8]> {
    behind_marker(icmp, if v6 { REPLY_V6 } else { REPLY_V4 }, marker).map(|(_id, p)| p)
}

fn behind_marker(icmp: &[u8], want_type: u8, marker: [u8; MARKER_LEN]) -> Option<(u16, &[u8])> {
    if icmp.len() < HEADER_LEN + MARKER_LEN {
        return None;
    }
    if icmp[0] != want_type {
        return None;
    }
    if icmp[HEADER_LEN..HEADER_LEN + MARKER_LEN] != marker {
        return None;
    }
    let id = u16::from_be_bytes([icmp[4], icmp[5]]);
    Some((id, &icmp[HEADER_LEN + MARKER_LEN..]))
}

pub struct Peeled<'a> {
    pub src: IpAddr,
    pub id: u16,
    pub payload: &'a [u8],
}

fn extract_echo(
    ethertype: u16,
    pkt: &[u8],
    v4: u8,
    v6: u8,
    marker: [u8; MARKER_LEN],
) -> Option<Peeled<'_>> {
    match ethertype {
        ETH_P_IP => {
            if pkt.len() < 20 || pkt[9] != PROTO_ICMP {
                return None;
            }
            let ihl = (pkt[0] & 0x0F) as usize * 4;
            if ihl < 20 {
                return None;
            }
            let (id, payload) = behind_marker(pkt.get(ihl..)?, v4, marker)?;
            let src = IpAddr::V4(std::net::Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]));
            Some(Peeled { src, id, payload })
        }
        ETH_P_IPV6 => {
            if pkt.len() < 40 || pkt[6] != PROTO_ICMPV6 {
                return None;
            }
            let (id, payload) = behind_marker(&pkt[40..], v6, marker)?;
            let raw: [u8; 16] = pkt[8..24].try_into().ok()?;
            let src = IpAddr::V6(std::net::Ipv6Addr::from(raw));
            Some(Peeled { src, id, payload })
        }
        _ => None,
    }
}

pub fn extract_request(ethertype: u16, pkt: &[u8], marker: [u8; MARKER_LEN]) -> Option<Peeled<'_>> {
    extract_echo(ethertype, pkt, REQUEST_V4, REQUEST_V6, marker)
}

#[cfg(test)]
mod tests;
