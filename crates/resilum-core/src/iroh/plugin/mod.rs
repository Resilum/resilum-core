//! Announce-driven warm discovery: advertise our `EndpointId`(+relay) and dial
//! peers that advertise theirs on the `resilum.discovery.iroh` aspect. Dormant
//! until the transport attaches — the endpoint exists only then.

mod addr;

use std::sync::{Arc, Mutex};

use iroh::Endpoint;
use leviculum_std::driver::ReticulumNode;

use addr::{encode_addr, parse_addr};

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
