use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, DestinationHash, Identity};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc;

use crate::coordinates::{Coordinates, PeerId, exchange};
use crate::link::{self, Inbound, LinkRouter};
use crate::node::Node;
use crate::wall_clock;

const ASK_EVERY: Duration = Duration::from_secs(300);
const ANNOUNCE_EVERY: Duration = Duration::from_secs(600);
const FORGET_AFTER: f64 = 24.0 * 60.0 * 60.0;

pub(super) fn bring_up(
    node: &mut Node,
    engine: &Arc<ReticulumNode>,
    identity: &Identity,
    router: &Arc<LinkRouter>,
    inbox: &Arc<link::Inbox>,
    arriving: mpsc::UnboundedReceiver<Inbound>,
    unclaimed: mpsc::UnboundedSender<Inbound>,
) {
    let destination = exchange::destination(identity.clone());
    let ours = *destination.hash();
    engine.register_destination(destination);
    let asked_of_us = inbox.claim(*ours.as_bytes());
    node.tasks.push(tokio::spawn({
        let inbox = inbox.clone();
        async move { inbox.sort(arriving, unclaimed).await }
    }));
    node.tasks.push(tokio::spawn(exchange::answer(
        engine.clone(),
        asked_of_us,
        node.coordinates.clone(),
        wall_clock::unix_now,
    )));
    node.tasks
        .push(tokio::spawn(announce(engine.clone(), ours)));
    node.tasks.push(tokio::spawn(ask_around(
        engine.clone(),
        router.clone(),
        node.coordinates.clone(),
    )));
}

async fn announce(engine: Arc<ReticulumNode>, ours: DestinationHash) {
    loop {
        let _ = engine.announce_destination(&ours, None).await;
        tokio::time::sleep(ANNOUNCE_EVERY).await;
    }
}

async fn ask_around(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    coordinates: Arc<Coordinates>,
) {
    let aspect = Destination::compute_name_hash(exchange::APP_NAME, &[exchange::ASPECT]);
    loop {
        tokio::time::sleep(ASK_EVERY).await;
        let now = wall_clock::unix_now();
        coordinates.forget_before(now - FORGET_AFTER);
        let asking = reachable_peers(&engine, &aspect);
        let mut placed = 0;
        for (peer, at) in &asking {
            if exchange::place(&engine, &router, &coordinates, *peer, *at, now).await {
                placed += 1;
            }
        }
        tracing::debug!(
            asked = asking.len(),
            placed,
            ours = %format_args!("{:?}", coordinates.ours()),
            "asked the peers this node can reach where they sit"
        );
    }
}

fn reachable_peers(
    engine: &Arc<ReticulumNode>,
    aspect: &[u8; 10],
) -> Vec<(PeerId, DestinationHash)> {
    let mut asking = Vec::new();
    for entry in engine.path_table_entries() {
        let Some(identity) = engine.get_identity(&entry.hash.into()) else {
            continue;
        };
        let at = Destination::compute_destination_hash(aspect, identity.hash());
        if engine.has_path(&at) {
            asking.push((*identity.hash(), at));
        }
    }
    asking.sort_unstable();
    asking.dedup();
    asking
}
