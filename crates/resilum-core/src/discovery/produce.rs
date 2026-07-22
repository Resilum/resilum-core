use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::Node as LevNode;
use leviculum_std::{Destination, DestinationHash, DestinationType, Direction, Identity};
use tokio::sync::Notify;

use super::{APP_NAME, Discovery};
use crate::announce_payload;
use crate::config::DiscoveryService;
use crate::error::{Error, Result};

/// One `resilum.discovery.<service>` destination per configured plugin,
/// registered with the engine so incoming announces route to it. Returned as
/// `(service, dest_hash)` pairs for the produce loop to key its per-service
/// announces off.
pub fn build_destinations(
    engine: &LevNode,
    identity: Identity,
    services: &[DiscoveryService],
) -> Result<Vec<(String, DestinationHash)>> {
    let mut out = Vec::with_capacity(services.len());
    for cfg in services {
        let dest = Destination::new(
            Some(identity.clone()),
            Direction::In,
            DestinationType::Single,
            APP_NAME,
            &["discovery", &cfg.service],
        )
        .map_err(|e| Error::Engine(format!("discovery destination for {}: {e}", cfg.service)))?;
        let hash = *dest.hash();
        engine.register_destination(dest);
        out.push((cfg.service.clone(), hash));
    }
    Ok(out)
}

/// Periodic produce: announce each ready plugin's endpoint. Runs once at
/// startup for immediate discoverability, then re-announces every `interval`
/// or whenever `trigger.notify_waiters()` fires (e.g. Flutter posts a
/// network-change event through FFI).
pub async fn run_produce(
    engine: Arc<LevNode>,
    discovery: Arc<Discovery>,
    destinations: Vec<(String, DestinationHash)>,
    interval: Duration,
    trigger: Arc<Notify>,
) {
    let mut ticker = tokio::time::interval(interval);
    // interval fires immediately on the first tick, giving discoverability
    // without waiting a full period.
    loop {
        announce_all(&engine, &discovery, &destinations).await;
        tokio::select! {
            _ = ticker.tick() => {}
            _ = trigger.notified() => {}
        }
    }
}

async fn announce_all(
    engine: &LevNode,
    discovery: &Discovery,
    destinations: &[(String, DestinationHash)],
) {
    let ready: std::collections::HashMap<_, _> = discovery.endpoints().into_iter().collect();
    for (service, dest_hash) in destinations {
        let Some(endpoint) = ready.get(service) else {
            continue;
        };
        let packed = announce_payload::pack(Some(endpoint), "*", &[]);
        if let Err(e) = engine.announce(dest_hash, Some(&packed)).await {
            tracing::warn!(service = %service, error = %e, "announce failed");
        }
    }
}
