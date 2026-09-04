mod pace;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use pace::{between_asks, forgotten_after, still_resting};

use leviculum_std::api::{Destination, DestinationHash, Identity};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc;

use crate::coordinates::{Coordinates, PeerId, exchange};
use crate::discovery::Attachments;
use crate::link::{self, Inbound, LinkRouter};
use crate::node::Node;
use crate::wall_clock;

const ANNOUNCE_EVERY: Duration = Duration::from_secs(600);

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
        node.nursery.clone(),
    )));
    node.tasks.push(tokio::spawn(crate::announce_ours::every(
        engine.clone(),
        ours,
        ANNOUNCE_EVERY,
    )));
    node.tasks.push(tokio::spawn(ask_around(
        engine.clone(),
        router.clone(),
        node.coordinates.clone(),
        node.attachments.clone(),
        identity.clone(),
    )));
}

async fn ask_around(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    coordinates: Arc<Coordinates>,
    attachments: Arc<Attachments>,
    us: Identity,
) {
    let aspect = Destination::compute_name_hash(exchange::APP_NAME, &[exchange::ASPECT]);
    let mut round = 0usize;
    let mut asked_at: HashMap<PeerId, f64> = HashMap::new();
    loop {
        let between_asks = between_asks(coordinates.how_wrong_we_are());
        tokio::time::sleep(between_asks).await;
        let now = wall_clock::unix_now();
        coordinates.forget_before(now - forgotten_after(between_asks).as_secs_f64());
        let asking = whom_to_ask(&engine, &aspect, &attachments, round);
        round = round.wrapping_add(1);
        let mut placed = 0;
        let mut spared = 0;
        for (peer, at) in &asking {
            if still_resting(
                coordinates.estimated_rtt(peer),
                asked_at.get(peer).copied(),
                now,
            ) {
                spared += 1;
                continue;
            }
            asked_at.insert(*peer, now);
            if exchange::place(&engine, &router, &coordinates, &us, *peer, *at, now).await {
                placed += 1;
            }
        }
        asked_at.retain(|peer, _| asking.iter().any(|(asked, _)| asked == peer));
        tracing::debug!(
            asked = asking.len() - spared,
            spared,
            placed,
            how_wrong_we_are = coordinates.how_wrong_we_are(),
            ours = %format_args!("{:?}", coordinates.ours()),
            "asked the peers this node can reach where they sit"
        );
    }
}

fn whom_to_ask(
    engine: &Arc<ReticulumNode>,
    aspect: &[u8; 10],
    attachments: &Attachments,
    round: usize,
) -> Vec<(PeerId, DestinationHash)> {
    let kept = attachments.whose_links_we_keep();
    let (mut whose_links_we_keep, rest): (Vec<_>, Vec<_>) = reachable_peers(engine, aspect)
        .into_iter()
        .partition(|(peer, _)| kept.contains(peer));
    if whose_links_we_keep.is_empty() {
        return rest;
    }
    whose_links_we_keep.extend(one_of_the_rest_in_turn(&rest, round));
    whose_links_we_keep
}

fn one_of_the_rest_in_turn(
    rest: &[(PeerId, DestinationHash)],
    round: usize,
) -> Option<(PeerId, DestinationHash)> {
    rest.get(round % rest.len().max(1)).copied()
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
