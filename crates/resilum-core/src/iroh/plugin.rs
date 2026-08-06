//! Announce-driven warm discovery: advertise our `EndpointId`(+relay) and dial
//! peers that advertise theirs on the `resilum.discovery.iroh` aspect. Dormant
//! until the transport attaches — the endpoint exists only then.

use std::sync::{Arc, Mutex};

use iroh::{Endpoint, EndpointAddr, EndpointId, TransportAddr};
use leviculum_std::api::Node as LevNode;

use super::{Links, dial};
use crate::discovery::DiscoveryPlugin;

struct Active {
    endpoint: Endpoint,
    engine: Arc<LevNode>,
    links: Links,
}

#[derive(Default)]
pub struct IrohDiscovery {
    active: Mutex<Option<Active>>,
}

impl IrohDiscovery {
    /// Wire the live transport in, so announces start producing and consuming.
    pub fn activate(&self, endpoint: Endpoint, engine: Arc<LevNode>, links: Links) {
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

/// `EndpointId` (32 bytes) followed by the relay URL, if any — all a peer needs
/// to route to us without a DNS lookup.
fn encode_addr(addr: &EndpointAddr) -> Vec<u8> {
    let mut out = addr.id.as_bytes().to_vec();
    if let Some(relay) = addr.addrs.iter().find_map(|a| match a {
        TransportAddr::Relay(url) => Some(url.to_string()),
        _ => None,
    }) {
        out.extend_from_slice(relay.as_bytes());
    }
    out
}

fn parse_addr(payload: &[u8]) -> Option<EndpointAddr> {
    let id_bytes: [u8; 32] = payload.get(..32)?.try_into().ok()?;
    let id = EndpointId::from_bytes(&id_bytes).ok()?;
    let mut addr = EndpointAddr::new(id);
    if payload.len() > 32
        && let Ok(text) = std::str::from_utf8(&payload[32..])
        && let Ok(url) = text.parse()
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
