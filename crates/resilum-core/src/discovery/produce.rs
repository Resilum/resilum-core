use std::sync::Arc;
use std::time::Duration;

use leviculum_std::driver::ReticulumNode;
use leviculum_std::{Destination, DestinationHash, DestinationType, Direction, Identity};
use tokio::sync::Notify;

use super::{APP_NAME, Discovery};
use crate::announce_payload;
use crate::error::{Error, Result};

/// Fill up to what an announce can carry rather than be refused for overrunning
/// it. Ratcheted announces have less room, so that is the figure to respect.
fn app_data_budget() -> usize {
    leviculum_core::announce_app_data_budget(true)
}

/// The single `resilum.discovery` destination every transport announces on.
///
/// An announce costs about 170 bytes of key, hashes and signature before any
/// payload, so one per service spends more on framing than on endpoints.
pub fn build_destination(engine: &ReticulumNode, identity: Identity) -> Result<DestinationHash> {
    let dest = Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        &["discovery"],
    )
    .map_err(|e| Error::Engine(format!("discovery destination: {e}")))?;
    let hash = *dest.hash();
    engine.register_destination(dest);
    Ok(hash)
}

/// Periodic produce: one announce carrying every ready endpoint. Runs once at
/// startup for immediate discoverability, then re-announces every `interval`
/// or whenever `trigger.notify_waiters()` fires (an embedder reporting a
/// network change, say).
pub async fn run_produce(
    engine: Arc<ReticulumNode>,
    discovery: Arc<Discovery>,
    destination: DestinationHash,
    interval: Duration,
    trigger: Arc<Notify>,
) {
    on_tick_or_trigger(interval, trigger, || {
        announce_all(&engine, &discovery, &destination)
    })
    .await;
}

pub async fn on_tick_or_trigger<F, Fut>(interval: Duration, trigger: Arc<Notify>, mut announce: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut ticker = tokio::time::interval(interval);
    loop {
        announce().await;
        tokio::select! {
            _ = ticker.tick() => {}
            _ = trigger.notified() => {}
        }
    }
}

async fn announce_all(
    engine: &ReticulumNode,
    discovery: &Discovery,
    destination: &DestinationHash,
) {
    let ready = discovery.endpoints();
    if ready.is_empty() {
        tracing::debug!("no transport is ready, skipping announce");
        return;
    }
    let (packed, left_out) = announce_payload::pack(&ready, "*", app_data_budget());
    if !left_out.is_empty() {
        tracing::warn!(
            services = ?left_out,
            "announce is full; these transports are not advertised",
        );
    }
    let announced: Vec<&str> = ready
        .keys()
        .map(String::as_str)
        .filter(|s| !left_out.iter().any(|out| out == s))
        .collect();
    match engine
        .announce_destination(destination, Some(&packed))
        .await
    {
        Ok(()) => tracing::debug!(services = ?announced, bytes = packed.len(), "announced"),
        Err(e) => tracing::warn!(error = %e, "announce failed"),
    }
}
