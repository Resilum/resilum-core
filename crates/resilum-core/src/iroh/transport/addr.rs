//! `CustomAddr` ↔ `SocketAddr` mapping for the protected UDP transport: a tag
//! byte (4/6), the IP octets, then the port big-endian.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use iroh_base::CustomAddr;

/// Our transport id in iroh's `CustomAddr` namespace.
pub(super) const TRANSPORT_ID: u64 = 0x2020;

pub(super) fn to_custom(addr: SocketAddr) -> CustomAddr {
    let mut data = Vec::with_capacity(19);
    match addr.ip() {
        IpAddr::V4(ip) => {
            data.push(4);
            data.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            data.push(6);
            data.extend_from_slice(&ip.octets());
        }
    }
    data.extend_from_slice(&addr.port().to_be_bytes());
    CustomAddr::from_parts(TRANSPORT_ID, &data)
}

pub(super) fn from_custom(addr: &CustomAddr) -> Option<SocketAddr> {
    if addr.id() != TRANSPORT_ID {
        return None;
    }
    let data = addr.data();
    let (ip, rest): (IpAddr, &[u8]) = match data.first()? {
        4 => (
            Ipv4Addr::from(<[u8; 4]>::try_from(data.get(1..5)?).ok()?).into(),
            data.get(5..)?,
        ),
        6 => (
            Ipv6Addr::from(<[u8; 16]>::try_from(data.get(1..17)?).ok()?).into(),
            data.get(17..)?,
        ),
        _ => return None,
    };
    let port = u16::from_be_bytes(<[u8; 2]>::try_from(rest.get(..2)?).ok()?);
    Some(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_v4_and_v6() {
        for s in ["203.0.113.7:4242", "[2001:db8::1]:9000"] {
            let addr: SocketAddr = s.parse().unwrap();
            assert_eq!(from_custom(&to_custom(addr)), Some(addr));
        }
    }

    #[test]
    fn rejects_foreign_transport_id() {
        let foreign = CustomAddr::from_parts(0x99, &[4, 203, 0, 113, 7, 0x10, 0x92]);
        assert!(from_custom(&foreign).is_none());
    }
}
