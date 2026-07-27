//! Covert-carrier discovery plugin. On peer announce → fetch endpoint over an
//! encrypted rendezvous link → attach a per-peer covert interface in-process.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use leviculum_std::api::Node as LevNode;
use leviculum_std::interfaces::ByteChannelHandle;

use super::super::DiscoveryPlugin;
use super::AddressSource;
use super::rendezvous;
use crate::config::CovertDiscoveryService;
use crate::dispatch::Events;

pub struct CovertDiscovered {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: CovertDiscoveryService,
    addresses: Arc<AddressSource>,
    engine: Arc<LevNode>,
    events: Events,
    attached: Mutex<HashMap<Vec<u8>, ByteChannelHandle>>,
    origin_registry: Arc<crate::discovery::OriginRegistry>,
}

impl CovertDiscovered {
    pub fn new(
        cfg: CovertDiscoveryService,
        addresses: Arc<AddressSource>,
        engine: Arc<LevNode>,
        events: Events,
        origin_registry: Arc<crate::discovery::OriginRegistry>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                cfg,
                addresses,
                engine,
                events,
                attached: Mutex::new(HashMap::new()),
                origin_registry,
            }),
        }
    }
}

impl DiscoveryPlugin for CovertDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        (!self.inner.addresses.effective().is_empty()).then(Vec::new)
    }

    fn consume_endpoint(&self, _payload: &[u8], announcer_pubkey: &[u8]) {
        let inner = Arc::clone(&self.inner);
        let pubkey = announcer_pubkey.to_vec();
        if inner
            .attached
            .lock()
            .expect("attached")
            .contains_key(&pubkey)
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
    let Some(addr) = addrs.into_iter().next() else {
        return;
    };
    let name = format!("CovertDiscovered[{carrier}:{addr}]");
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
            inner
                .attached
                .lock()
                .expect("attached")
                .insert(pubkey, handle);
        }
        Err(e) => {
            tracing::warn!(%name, error = %e, "covert peer attach failed");
        }
    }
}
