use std::sync::Arc;

use leviculum_std::NodeEvent;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;

use super::Discovery;
use crate::announce_payload;

/// Route each incoming `AnnounceReceived` to its plugin, unwrapping the JSON
/// envelope: app_data is `announce_payload::pack(endpoint)`, not the raw
/// endpoint. An envelope from an incompatible major or without an endpoint is
/// dropped silently.
pub async fn run_consume(discovery: Arc<Discovery>, mut rx: Receiver<Arc<NodeEvent>>) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let NodeEvent::AnnounceReceived { announce, .. } = &*event {
                    let Some(parsed) = announce_payload::parse(announce.app_data()) else {
                        continue;
                    };
                    discovery.on_announce(&parsed.endpoints, announce.public_key());
                }
            }
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return,
        }
    }
}
