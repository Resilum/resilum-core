//! How an endpoint travels in an announce.

use iroh::{EndpointAddr, EndpointId, TransportAddr};

/// The 32-byte endpoint id, then the relay's host if there is one. Raw rather
/// than hex, and the host without its scheme: both halved a field that shares
/// one announce with every other transport.
pub(super) fn encode_addr(addr: &EndpointAddr) -> Vec<u8> {
    let mut out = addr.id.as_bytes().to_vec();
    if let Some(host) = addr.addrs.iter().find_map(|a| match a {
        TransportAddr::Relay(url) => url.host_str().map(str::to_owned),
        _ => None,
    }) {
        out.extend_from_slice(host.as_bytes());
    }
    out
}

pub(super) fn parse_addr(payload: &[u8]) -> Option<EndpointAddr> {
    let (id_bytes, host) = payload.split_at_checked(iroh::PublicKey::LENGTH)?;
    let id_bytes: [u8; iroh::PublicKey::LENGTH] = id_bytes.try_into().ok()?;
    let mut addr = EndpointAddr::new(EndpointId::from_bytes(&id_bytes).ok()?);
    if !host.is_empty()
        && let Ok(host) = std::str::from_utf8(host)
        && let Ok(url) = format!("https://{host}/").parse()
    {
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

        let eps = std::collections::BTreeMap::from([("iroh".to_owned(), sent)]);
        let (packed, _) = crate::announce_payload::pack(
            &eps,
            "*",
            leviculum_core::announce_app_data_budget(true),
        );
        let received = crate::announce_payload::parse(&packed)
            .expect("envelope parses")
            .endpoints
            .remove("iroh")
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
