//! Announce-driven warm discovery: advertise our `EndpointId`(+relay) and dial
//! peers that advertise theirs on the `resilum.discovery.iroh` aspect. Dormant
//! until the transport attaches — the endpoint exists only then.

mod addr;

use std::sync::{Arc, Mutex};

use iroh::Endpoint;

use addr::{encode_addr, parse_addr};

use super::dial;
use super::wiring::{Wiring, attached_as};
use crate::discovery::DiscoveryPlugin;
use crate::discovery::admit::{self, Room};

struct Active {
    endpoint: Endpoint,
    wiring: Arc<Wiring>,
}

#[derive(Default)]
pub struct IrohDiscovery {
    active: Mutex<Option<Active>>,
}

impl IrohDiscovery {
    /// Wire the live transport in, so announces start producing and consuming.
    pub(super) fn activate(&self, endpoint: Endpoint, wiring: Arc<Wiring>) {
        *self.active.lock().unwrap_or_else(|e| e.into_inner()) = Some(Active { endpoint, wiring });
    }

    pub fn deactivate(&self) {
        *self.active.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl DiscoveryPlugin for IrohDiscovery {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        let guard = self.active.lock().unwrap_or_else(|e| e.into_inner());
        Some(encode_addr(&guard.as_ref()?.endpoint.addr()))
    }

    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: Option<&[u8]>) {
        let guard = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let Some(active) = guard.as_ref() else {
            return;
        };
        let Some(addr) = parse_addr(payload) else {
            return;
        };
        let name = attached_as(addr.id);
        let peer = admit::who_announced(announcer_pubkey.unwrap_or_default());
        if active.wiring.attachments.holds(&name) {
            if let Some(peer) = peer
                && active.wiring.attachments.learn_who_announced(&name, peer)
            {
                tracing::debug!(%name, "a peer that dialled us first has a name now");
            }
            return;
        }
        match admit::room_for(
            &active.wiring.attachments,
            active.wiring.engine.path_count(),
            &name,
            peer,
            admit::Reached::OverTheNetwork,
        ) {
            Room::Yes | Room::OnceThisIsLetGo(_) => {}
            Room::No => return,
        }
        tokio::spawn(dial::dial(
            active.endpoint.clone(),
            active.wiring.clone(),
            addr,
            peer,
        ));
    }
}
