//! Announce-driven warm discovery: advertise our `EndpointId`(+relay) and dial
//! peers that advertise theirs on the `resilum.discovery.iroh` aspect. Dormant
//! until the transport attaches — the endpoint exists only then.

use std::sync::{Arc, Mutex};

use iroh::{Endpoint, EndpointAddr, EndpointId, TransportAddr};
use leviculum_std::driver::ReticulumNode;

use super::{Links, dial};
use crate::discovery::DiscoveryPlugin;

struct Active {
    endpoint: Endpoint,
    engine: Arc<ReticulumNode>,
    links: Links,
}

#[derive(Default)]
pub struct IrohDiscovery {
    active: Mutex<Option<Active>>,
}

impl IrohDiscovery {
    /// Wire the live transport in, so announces start producing and consuming.
    pub fn activate(&self, endpoint: Endpoint, engine: Arc<ReticulumNode>, links: Links) {
        *self.active.lock().expect("iroh discovery") = Some(Active {
            endpoint,
            engine,
            links,
        });
    }

    pub fn deactivate(&self) {
        *self.active.lock().expect("iroh discovery") = None;
    }
}

impl DiscoveryPlugin for IrohDiscovery {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        let guard = self.active.lock().expect("iroh discovery");
        Some(encode_addr(&guard.as_ref()?.endpoint.addr()))
    }

    fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: &[u8]) {
        let guard = self.active.lock().expect("iroh discovery");
        let Some(active) = guard.as_ref() else {
            return;
        };
        let Some(addr) = parse_addr(payload) else {
            return;
        };
        if active
            .links
            .lock()
            .expect("iroh links")
            .contains_key(&addr.id)
        {
            return;
        }
        tokio::spawn(dial::dial(
            active.endpoint.clone(),
            active.engine.clone(),
            active.links.clone(),
            addr,
        ));
    }
}

/// `<hex endpoint id>[@<relay url>]` — all a peer needs to route to us without a
/// lookup. Text, because the announce envelope carries endpoints as UTF-8 (raw
/// key bytes would not survive it).
fn encode_addr(addr: &EndpointAddr) -> Vec<u8> {
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

fn parse_addr(payload: &[u8]) -> Option<EndpointAddr> {
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
