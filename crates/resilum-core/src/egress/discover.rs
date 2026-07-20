//! Populate the candidate registry from egress announces.

use std::sync::Arc;

use leviculum_std::NodeEvent;
use leviculum_std::api::Destination;
use tokio::sync::broadcast;

use crate::announce_payload;
use crate::egress::CandidateRegistry;

const APP_NAME: &str = "resilum";

/// Consume announces for `service`, upserting parsed peers into `registry` and
/// dropping peers whose announce no longer parses. Runs until the bus closes.
pub async fn run(
    registry: Arc<CandidateRegistry>,
    service: String,
    mut bus: broadcast::Receiver<Arc<NodeEvent>>,
) {
    let want = Destination::compute_name_hash(APP_NAME, &["bridge", "tcp", &service]);
    loop {
        match bus.recv().await {
            Ok(ev) => consume(&registry, &service, &want, &ev),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

fn consume(registry: &CandidateRegistry, service: &str, want: &[u8], ev: &NodeEvent) {
    let NodeEvent::AnnounceReceived { announce, .. } = ev else {
        return;
    };
    if announce.name_hash().as_slice() != want {
        return;
    }
    let dest_hash = announce.destination_hash().as_bytes().to_vec();
    match announce_payload::parse(announce.app_data()) {
        Some(p) => registry.upsert(service, dest_hash, &p.exit_country, p.capabilities),
        None => registry.remove(service, &dest_hash),
    }
}
