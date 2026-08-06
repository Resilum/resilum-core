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
        out.push(build_destination(engine, identity.clone(), &cfg.service)?);
    }
    Ok(out)
}

/// Register one `resilum.discovery.<service>` destination and return its
/// `(service, hash)` — used for services outside the `discovery` list (iroh).
pub fn build_destination(
    engine: &LevNode,
    identity: Identity,
    service: &str,
) -> Result<(String, DestinationHash)> {
    let dest = Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        &["discovery", service],
    )
    .map_err(|e| Error::Engine(format!("discovery destination for {service}: {e}")))?;
    let hash = *dest.hash();
    engine.register_destination(dest);
    Ok((service.to_owned(), hash))
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
            tracing::debug!(service = %service, "endpoint not ready, skipping announce");
            continue;
        };
        let packed = announce_payload::pack(Some(endpoint), "*", &[]);
        match engine.announce(dest_hash, Some(&packed)).await {
            Ok(()) => tracing::debug!(
                service = %service,
                endpoint = %String::from_utf8_lossy(endpoint),
                "announced",
            ),
            Err(e) => tracing::warn!(service = %service, error = %e, "announce failed"),
        }
    }
}
