//! Rendezvous responder: peers open a link to `resilum.discovery.covert_<carrier>`
//! and request the `endpoint` path; we reply with the packed local addresses.

use std::sync::Arc;

use leviculum_std::NodeEvent;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::{Destination, DestinationHash, DestinationType, Direction, Identity};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;

use super::super::super::APP_NAME;
use super::super::AddressSource;
use super::super::endpoint;
use super::ENDPOINT_PATH;
use crate::config::CovertDiscoveryService;
use crate::error::{Error, Result};

pub fn build_destinations(
    engine: &ReticulumNode,
    identity: Identity,
    covert: &[CovertDiscoveryService],
) -> Result<Vec<(String, DestinationHash)>> {
    let mut out = Vec::with_capacity(covert.len());
    for cfg in covert {
        let service = cfg.service_name();
        let dest = Destination::new(
            Some(identity.clone()),
            Direction::In,
            DestinationType::Single,
            APP_NAME,
            &["discovery", &service],
        )
        .map_err(|e| Error::Engine(format!("covert destination for {service}: {e}")))?;
        let hash = *dest.hash();
        engine.register_destination(dest);
        engine.register_request_handler(
            hash,
            ENDPOINT_PATH,
            leviculum_std::api::RequestPolicy::AllowAll,
        );
        out.push((service, hash));
    }
    Ok(out)
}

pub async fn run_announcer(
    engine: Arc<ReticulumNode>,
    destinations: Vec<DestinationHash>,
    interval: std::time::Duration,
    trigger: Arc<tokio::sync::Notify>,
) {
    crate::discovery::produce::on_tick_or_trigger(interval, trigger, || {
        give_peers_a_path_to_the_responder(&engine, &destinations)
    })
    .await;
}

async fn give_peers_a_path_to_the_responder(
    engine: &ReticulumNode,
    destinations: &[DestinationHash],
) {
    for hash in destinations {
        if let Err(e) = engine.announce_destination(hash, None).await {
            tracing::warn!(error = %e, "covert rendezvous announce failed");
        }
    }
}

/// One responder task per configured covert carrier. Fires on every
/// `RequestReceived` event on path `endpoint` and replies with our addresses.
pub async fn run_responder(
    engine: Arc<ReticulumNode>,
    carrier: String,
    addresses: Arc<AddressSource>,
    mut events: Receiver<Arc<NodeEvent>>,
) {
    loop {
        match events.recv().await {
            Ok(event) => {
                let NodeEvent::RequestReceived {
                    link_id,
                    request_id,
                    path,
                    ..
                } = &*event
                else {
                    continue;
                };
                if path != ENDPOINT_PATH {
                    continue;
                }
                let addrs = addresses.effective();
                if addrs.is_empty() {
                    continue;
                }
                let Ok(response) = super::as_one_msgpack_value(&endpoint::pack(&carrier, addrs))
                else {
                    continue;
                };
                if let Err(e) = engine.send_response(link_id, request_id, &response).await {
                    tracing::warn!(carrier = %carrier, error = %e, "rendezvous respond failed");
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        }
    }
}
