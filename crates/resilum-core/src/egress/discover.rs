//! Populate the candidate registry from egress announces, and drop (and tear
//! down active links to) peers whose announce stops parsing.

use std::collections::HashSet;
use std::sync::Arc;

use leviculum_std::NodeEvent;
use leviculum_std::api::{Destination, Node as LevNode};
use tokio::sync::broadcast;

use crate::Event;
use crate::announce_payload;
use crate::egress::{ActiveLinks, CandidateRegistry};
use crate::event::{self, Queue};

const APP_NAME: &str = "resilum";

pub async fn run(
    engine: Arc<LevNode>,
    registry: Arc<CandidateRegistry>,
    active: Arc<ActiveLinks>,
    events: Queue,
    service: String,
    skip: HashSet<Vec<u8>>,
    mut bus: broadcast::Receiver<Arc<NodeEvent>>,
) {
    let want = Destination::compute_name_hash(APP_NAME, &["bridge", "tcp", &service]);
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
        let dest_hash = announce.destination_hash().as_bytes();
        let dest_hex = data_encoding::HEXLOWER.encode(dest_hash.as_slice());
        if skip.contains(dest_hash.as_slice()) {
            tracing::debug!(service = %service, dest = %dest_hex, "own egress, skipping");
            continue;
        }
        match announce_payload::parse(announce.app_data()) {
            Some(p) => {
                let added = registry.upsert(
                    &service,
                    dest_hash.to_vec(),
                    &p.exit_country,
                    p.capabilities,
                );
                tracing::debug!(service = %service, dest = %dest_hex, added, "egress candidate");
                if added {
                    event::push(&events, Event::PeerDiscovered(dest_hash.to_vec()));
                }
            }
            None => {
                tracing::debug!(service = %service, dest = %dest_hex, "unparseable egress announce");
                registry.remove(&service, dest_hash);
                active.teardown_for(&engine, dest_hash).await;
            }
        }
    }
}
