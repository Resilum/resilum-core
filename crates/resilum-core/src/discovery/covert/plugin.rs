//! Covert-carrier discovery plugin. On peer announce → fetch endpoint over an
//! encrypted rendezvous link → attach a per-peer PipeInterface.

use std::sync::Arc;

use leviculum_std::api::Node as LevNode;

use super::super::DiscoveryPlugin;
use super::rendezvous;
use crate::config::CovertDiscoveryService;
use crate::dispatch::Events;

pub struct CovertDiscovered {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: CovertDiscoveryService,
    engine: Arc<LevNode>,
    events: Events,
}

impl CovertDiscovered {
    pub fn new(cfg: CovertDiscoveryService, engine: Arc<LevNode>, events: Events) -> Self {
        Self {
            inner: Arc::new(Inner {
                cfg,
                engine,
                events,
            }),
        }
    }
}

impl DiscoveryPlugin for CovertDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        (!self.inner.cfg.addresses.is_empty()).then(Vec::new)
    }

    fn consume_endpoint(&self, _payload: &[u8], announcer_pubkey: &[u8]) {
        let inner = Arc::clone(&self.inner);
        let pubkey = announcer_pubkey.to_vec();
        tokio::spawn(async move {
            let events = inner.events.subscribe();
            let Some((carrier, addrs)) = rendezvous::fetch_endpoint(
                inner.engine.clone(),
                pubkey,
                inner.cfg.carrier.clone(),
                events,
            )
            .await
            else {
                tracing::debug!(carrier = %inner.cfg.carrier, "rendezvous fetch yielded no endpoint");
                return;
            };
            tracing::info!(carrier = %carrier, addrs = ?addrs, "discovered covert peer");
        });
    }
}
