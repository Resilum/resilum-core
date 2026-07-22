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
        if skip.contains(dest_hash.as_slice()) {
            continue;
        }
        match announce_payload::parse(announce.app_data()) {
            Some(p) => {
                if registry.upsert(
                    &service,
                    dest_hash.to_vec(),
                    &p.exit_country,
                    p.capabilities,
                ) {
                    event::push(&events, Event::PeerDiscovered(dest_hash.to_vec()));
                }
            }
            None => {
                registry.remove(&service, dest_hash);
                active.teardown_for(&engine, dest_hash).await;
            }
        }
    }
}
