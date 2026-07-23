//! Rendezvous client: open a link to a peer's `covert_<carrier>` destination,
//! request the `endpoint` path, parse the returned addresses.

use std::sync::Arc;
use std::time::Duration;

use leviculum_std::NodeEvent;
use leviculum_std::api::Node as LevNode;
use leviculum_std::{Destination, DestinationType, Direction, Identity, LinkId};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::time;

use super::super::super::APP_NAME;
use super::super::endpoint;
use super::{ENDPOINT_PATH, REQUEST_TIMEOUT_MS};

const LINK_WAIT: Duration = Duration::from_millis(REQUEST_TIMEOUT_MS);
const REQUEST_WAIT: Duration = Duration::from_millis(REQUEST_TIMEOUT_MS + 2_000);

/// Fetch a peer's covert addresses. Returns the parsed `(carrier, addresses)`
/// tuple on success, or `None` on link failure, timeout, or malformed reply.
pub async fn fetch_endpoint(
    engine: Arc<LevNode>,
    announcer_pubkey: Vec<u8>,
    carrier: String,
    events: Receiver<Arc<NodeEvent>>,
) -> Option<(String, Vec<String>)> {
    let identity = Identity::from_public_key_bytes(&announcer_pubkey).ok()?;
    let signing_key: [u8; 32] = announcer_pubkey[32..64].try_into().ok()?;
    let dest = Destination::new(
        Some(identity),
        Direction::Out,
        DestinationType::Single,
        APP_NAME,
        &["discovery", &format!("covert_{carrier}")],
    )
    .ok()?;
    let dest_hash = *dest.hash();

    let handle = engine
        .connect_with_key(&dest_hash, &signing_key)
        .await
        .ok()?;
    let link_id = *handle.link_id();

    let mut events = events;
    if !wait_link_up(&mut events, link_id).await {
        return None;
    }
    let request_id = engine
        .send_request(&link_id, ENDPOINT_PATH, None, Some(REQUEST_TIMEOUT_MS))
        .await
        .ok()?;
    let raw = wait_response(&mut events, link_id, &request_id).await?;
    endpoint::parse(&raw)
}

async fn wait_link_up(events: &mut Receiver<Arc<NodeEvent>>, link_id: LinkId) -> bool {
    time::timeout(LINK_WAIT, async {
        loop {
            match events.recv().await {
                Ok(event) => match &*event {
                    NodeEvent::LinkEstablished { link_id: id, .. } if *id == link_id => {
                        return true;
                    }
                    NodeEvent::LinkClosed { link_id: id, .. } if *id == link_id => return false,
                    _ => continue,
                },
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return false,
            }
        }
    })
    .await
    .unwrap_or(false)
}

async fn wait_response(
    events: &mut Receiver<Arc<NodeEvent>>,
    link_id: LinkId,
    request_id: &[u8; 16],
) -> Option<Vec<u8>> {
    time::timeout(REQUEST_WAIT, async {
        loop {
            match events.recv().await {
                Ok(event) => match &*event {
                    NodeEvent::ResponseReceived {
                        link_id: lid,
                        request_id: rid,
                        response_data,
                        ..
                    } if *lid == link_id && rid == request_id => {
                        return Some(response_data.clone());
                    }
                    NodeEvent::LinkClosed { link_id: id, .. } if *id == link_id => return None,
                    _ => continue,
                },
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    })
    .await
    .unwrap_or(None)
}
