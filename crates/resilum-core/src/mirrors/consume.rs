use std::sync::Arc;

use leviculum_std::NodeEvent;
use tokio::sync::broadcast;

use super::Registry;
use super::payload::{name_hash, parse};

pub async fn run(registry: Arc<Registry>, mut bus: broadcast::Receiver<Arc<NodeEvent>>) {
    let want = name_hash();
    loop {
        let ev = match bus.recv().await {
            Ok(ev) => ev,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let NodeEvent::AnnounceReceived { announce, .. } = &*ev else {
            continue;
        };
        if announce.name_hash().as_slice() != want {
            continue;
        }
        let Some(advert) = parse(announce.app_data()) else {
            continue;
        };
        let peer_hex = data_encoding::HEXLOWER.encode(announce.destination_hash().as_ref());
        tracing::debug!(peer = %peer_hex, rngit = %advert.rngit, repos = ?advert.repos, "mirror advert accepted");
        registry.upsert(peer_hex, advert.rngit, advert.repos);
    }
}
