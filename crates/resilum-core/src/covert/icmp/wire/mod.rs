//! Build and parse ICMP/ICMPv6 echo messages carrying the wire datagram in the
//! echo data field. Building emits the ICMP body alone; the kernel adds the IP
//! header on send (and fills the ICMPv6 checksum). Parsing takes the L3 IP
//! packet plus its ethertype.

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

fn echo(icmp_type: u8, ident: u16, payload: &[u8], v6: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(HEADER_LEN + payload.len());
    buf.push(icmp_type);
    buf.push(0); // code
    buf.extend_from_slice(&[0, 0]); // checksum placeholder
    buf.extend_from_slice(&ident.to_be_bytes());
    buf.extend_from_slice(&[0, 0]); // seq
    buf.extend_from_slice(payload);
    if !v6 {
        // Kernel fills the ICMPv6 checksum; only IPv4 echoes need it here.
        let ck = checksum(&buf).to_be_bytes();
        buf[2] = ck[0];
        buf[3] = ck[1];
    }
    buf
}

pub fn build_echo_request(ident: u16, payload: &[u8], v6: bool) -> Vec<u8> {
    let ty = if v6 { REQUEST_V6 } else { REQUEST_V4 };
    echo(ty, ident, payload, v6)
}

pub fn build_echo_reply(ident: u16, payload: &[u8], v6: bool) -> Vec<u8> {
    let ty = if v6 { REPLY_V6 } else { REPLY_V4 };
    echo(ty, ident, payload, v6)
}

/// Payload from an ICMP echo-reply body (no IP header), matching only our id.
/// Used with `SOCK_DGRAM/IPPROTO_ICMP` where the kernel strips the IP header.
pub fn payload_of_reply(icmp: &[u8], ident: u16, v6: bool) -> Option<&[u8]> {
    echo_payload(icmp, if v6 { REPLY_V6 } else { REPLY_V4 }, ident)
}

/// Split an ICMP echo body into (id, payload). Returns `None` when the buffer
/// is too short, the ICMP type does not match, or the id does not match ours.
fn echo_payload(icmp: &[u8], want_type: u8, ident: u16) -> Option<&[u8]> {
    if icmp.len() < HEADER_LEN {
        return None;
    }
    if icmp[0] != want_type {
        return None;
    }
    let iid = u16::from_be_bytes([icmp[4], icmp[5]]);
    if iid != ident {
        return None;
    }
    Some(&icmp[HEADER_LEN..])
}

/// Src IP + payload for an inbound raw L3 packet.
pub struct Peeled<'a> {
    pub src: std::net::IpAddr,
    pub payload: &'a [u8],
}

/// Extract `(src, payload)` from the L3 packet whose ethertype is `ethertype`
/// (`ETH_P_IP` / `ETH_P_IPV6`), matching only echoes with our id.
pub fn extract_echo(
    ethertype: u16,
    pkt: &[u8],
    v4_type: u8,
    v6_type: u8,
    ident: u16,
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
            let payload = echo_payload(pkt.get(ihl..)?, v4_type, ident)?;
            let src =
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]));
            Some(Peeled { src, payload })
        }
        ETH_P_IPV6 => {
            if pkt.len() < 40 || pkt[6] != PROTO_ICMPV6 {
                return None;
            }
            let payload = echo_payload(&pkt[40..], v6_type, ident)?;
            let raw: [u8; 16] = pkt[8..24].try_into().ok()?;
            let src = std::net::IpAddr::V6(std::net::Ipv6Addr::from(raw));
            Some(Peeled { src, payload })
        }
        _ => None,
    }
}

pub fn extract_request(ethertype: u16, pkt: &[u8], ident: u16) -> Option<Peeled<'_>> {
    extract_echo(ethertype, pkt, REQUEST_V4, REQUEST_V6, ident)
}

pub fn extract_reply(ethertype: u16, pkt: &[u8], ident: u16) -> Option<Peeled<'_>> {
    extract_echo(ethertype, pkt, REPLY_V4, REPLY_V6, ident)
}

#[cfg(test)]
mod tests;
