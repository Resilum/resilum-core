//! Covert-carrier discovery plugin. On peer announce → fetch endpoint over an
//! encrypted rendezvous link → attach a per-peer covert interface in-process.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use leviculum_std::driver::ReticulumNode;

use super::super::DiscoveryPlugin;
use super::rendezvous;
use super::{AddressSource, DialableAddress};
use crate::config::CovertDiscoveryService;
use crate::discovery::admit::{self, Room};
use crate::discovery::attachments::{Attached, Attachments};
use crate::dispatch::Events;

pub struct CovertDiscovered {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: CovertDiscoveryService,
    addresses: Arc<AddressSource>,
    engine: Arc<ReticulumNode>,
    events: Events,
    dialled: Mutex<HashSet<Vec<u8>>>,
    origin_registry: Arc<crate::discovery::OriginRegistry>,
    attachments: Arc<Attachments>,
}

impl CovertDiscovered {
    pub fn new(
        cfg: CovertDiscoveryService,
        addresses: Arc<AddressSource>,
        engine: Arc<ReticulumNode>,
        events: Events,
        origin_registry: Arc<crate::discovery::OriginRegistry>,
        attachments: Arc<Attachments>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                cfg,
                addresses,
                engine,
                events,
                dialled: Mutex::new(HashSet::new()),
                origin_registry,
                attachments,
            }),
        }
    }
}

impl DiscoveryPlugin for CovertDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        (!self.inner.addresses.effective().is_empty()).then(Vec::new)
    }

    /// A covert session is sealed to the announcer, so an endpoint with no
    /// signer to name — one restored from the cache — is unusable here.
    fn consume_endpoint(&self, _payload: &[u8], announcer_pubkey: Option<&[u8]>) {
        let Some(pubkey) = announcer_pubkey else {
            return;
        };
        let inner = Arc::clone(&self.inner);
        let pubkey = pubkey.to_vec();
        if !inner
            .dialled
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(pubkey.clone())
        {
            return;
        }
        tokio::spawn(async move { resolve_and_attach(inner, pubkey).await });
    }
}

async fn resolve_and_attach(inner: Arc<Inner>, pubkey: Vec<u8>) {
    let events = inner.events.subscribe();
    let Some((carrier, addrs)) = rendezvous::fetch_endpoint(
        inner.engine.clone(),
        pubkey.clone(),
        inner.cfg.carrier.clone(),
        events,
    )
    .await
    else {
        tracing::debug!(carrier = %inner.cfg.carrier, "rendezvous fetch yielded no endpoint");
        return;
    };
    let Some(addr) = DialableAddress::first_of(&addrs, inner.cfg.reach()) else {
        tracing::warn!(
            carrier = %carrier,
            offered = ?addrs,
            "peer named no address this node may dial"
        );
        return;
    };
    let name = format!("CovertDiscovered[{carrier}:{addr}]");
    let peer = admit::who_announced(&pubkey);
    match admit::room_for(&inner.attachments, inner.engine.path_count(), &name, peer) {
        Room::Yes | Room::OnceThisIsLetGo(_) => {}
        Room::No => {
            inner.dialled.lock().expect("dialled").remove(&pubkey);
            return;
        }
    }
    match super::inproc::attach(
        &inner.engine,
        &name,
        &carrier,
        &addr,
        &pubkey,
        inner.cfg.mtu,
    ) {
        Ok(handle) => {
            tracing::info!(%name, carrier = %carrier, addr = %addr, "covert peer attached");
            inner.origin_registry.record(handle.id(), "covert");
            inner.attachments.hold(
                name,
                Attached {
                    service: inner.cfg.service_name(),
                    announced_by: peer,
                    interface: handle.id(),
                    _detaches_when_dropped: Box::new(handle),
                },
            );
        }
        Err(e) => {
            tracing::warn!(%name, error = %e, "covert peer attach failed");
            inner.dialled.lock().expect("dialled").remove(&pubkey);
        }
    }
}
