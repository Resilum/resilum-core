//! How an endpoint travels in an announce.

use iroh::{EndpointAddr, EndpointId, TransportAddr};

/// `<hex endpoint id>[@<relay url>]` — all a peer needs to route to us without a
/// lookup. Text, because the announce envelope carries endpoints as UTF-8 (raw
/// key bytes would not survive it).
pub(super) fn encode_addr(addr: &EndpointAddr) -> Vec<u8> {
    let mut out = data_encoding::HEXLOWER.encode(addr.id.as_bytes());
    if let Some(relay) = addr.addrs.iter().find_map(|a| match a {
        TransportAddr::Relay(url) => Some(url.to_string()),
        _ => None,
    }) {
        out.push('@');
        out.push_str(&relay);
    }
    out.into_bytes()
}

pub(super) fn parse_addr(payload: &[u8]) -> Option<EndpointAddr> {
    let text = std::str::from_utf8(payload).ok()?;
    let (id_hex, relay) = match text.split_once('@') {
        Some((id, relay)) => (id, Some(relay)),
        None => (text, None),
    };
    let id_bytes: [u8; 32] = data_encoding::HEXLOWER
        .decode(id_hex.as_bytes())
        .ok()?
        .try_into()
        .ok()?;
    let mut addr = EndpointAddr::new(EndpointId::from_bytes(&id_bytes).ok()?);
    if let Some(url) = relay.and_then(|r| r.parse().ok()) {
        addr = addr.with_relay_url(url);
    }
    Some(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn some_id() -> EndpointId {
        SecretKey::generate().public()
    }

    #[test]
    fn roundtrips_id_only() {
        let id = some_id();
        let back = parse_addr(&encode_addr(&EndpointAddr::new(id))).unwrap();
        assert_eq!(back.id, id);
        assert!(back.addrs.is_empty());
    }

    /// The real wire path: a direct encode/parse roundtrip stays green even when
    /// the envelope in between destroys the address.
    #[test]
    fn survives_the_announce_envelope() {
        let id = some_id();
        let url: iroh::RelayUrl = "https://relay.example./".parse().unwrap();
        let sent = encode_addr(&EndpointAddr::new(id).with_relay_url(url.clone()));

        let packed = crate::announce_payload::pack(Some(&sent), "*", &[]);
        let received = crate::announce_payload::parse(&packed)
            .expect("envelope parses")
            .endpoint
            .expect("endpoint present");

        let back = parse_addr(&received).expect("addr parses after the envelope");
        assert_eq!(back.id, id);
        assert!(
            back.addrs
                .iter()
                .any(|a| matches!(a, TransportAddr::Relay(u) if *u == url)),
            "relay url must survive, else iroh has no way to route",
        );
    }

    #[test]
    fn roundtrips_with_relay() {
        let id = some_id();
        let url: iroh::RelayUrl = "https://relay.example./".parse().unwrap();
        let bytes = encode_addr(&EndpointAddr::new(id).with_relay_url(url.clone()));
        let back = parse_addr(&bytes).unwrap();
        assert_eq!(back.id, id);
        assert!(
            back.addrs
                .iter()
                .any(|a| matches!(a, TransportAddr::Relay(u) if *u == url))
        );
    }

    #[test]
    fn rejects_short_payload() {
        assert!(parse_addr(&[0u8; 8]).is_none());
    }
}
