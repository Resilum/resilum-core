use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, DestinationHash, Identity};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc;

use crate::coordinates::{Coordinates, PeerId, exchange};
use crate::link::{self, Inbound, LinkRouter};
use crate::node::Node;
use crate::wall_clock;

const WHILE_SETTLING: Duration = Duration::from_secs(20);
const ONCE_SETTLED: Duration = Duration::from_secs(900);
const SETTLED_BELOW: f64 = 0.25;
const ROUNDS_MISSED_BEFORE_FORGOTTEN: u32 = 3;
const NEVER_FORGET_SOONER_THAN: Duration = Duration::from_secs(600);
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
        let between_asks = between_asks(coordinates.how_wrong_we_are());
        tokio::time::sleep(between_asks).await;
        let now = wall_clock::unix_now();
        coordinates.forget_before(now - forgotten_after(between_asks).as_secs_f64());
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

fn between_asks(our_error: f64) -> Duration {
    if our_error > SETTLED_BELOW {
        WHILE_SETTLING
    } else {
        ONCE_SETTLED
    }
}

fn forgotten_after(between_asks: Duration) -> Duration {
    (between_asks * ROUNDS_MISSED_BEFORE_FORGOTTEN).max(NEVER_FORGET_SOONER_THAN)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::Coordinates;

    #[test]
    fn a_node_that_has_just_started_asks_often_enough_to_settle_within_minutes() {
        let fresh = Coordinates::default().how_wrong_we_are();

        let rounds_to_settle = 40;
        let settling = between_asks(fresh) * rounds_to_settle;

        assert!(
            settling < Duration::from_secs(20 * 60),
            "settling would take {settling:?}"
        );
    }

    #[test]
    fn a_settled_node_leaves_the_mesh_alone() {
        assert_eq!(between_asks(SETTLED_BELOW / 2.0), ONCE_SETTLED);
    }

    #[test]
    fn a_peer_is_forgotten_after_the_rounds_it_missed_but_never_after_just_one() {
        assert_eq!(forgotten_after(ONCE_SETTLED), ONCE_SETTLED * 3);
        assert_eq!(forgotten_after(WHILE_SETTLING), NEVER_FORGET_SOONER_THAN);
    }
}
