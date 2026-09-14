mod pace;
mod whom;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, Identity};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc;

use self::pace::{between_asks, forgotten_after, still_resting};
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
    node.tasks
        .keep(resilum_tasks::watch("inbox: sorting what arrives", {
            let inbox = inbox.clone();
            async move { inbox.sort(arriving, unclaimed).await }
        }));
    node.tasks.keep(resilum_tasks::watch(
        "coordinates: answering an exchange",
        exchange::answer(
            engine.clone(),
            asked_of_us,
            node.coordinates.clone(),
            wall_clock::unix_now,
            node.nursery.clone(),
        ),
    ));
    node.tasks.keep(resilum_tasks::watch(
        "coordinates: announcing where to reach us",
        crate::announce_ours::every(engine.clone(), ours, ANNOUNCE_EVERY),
    ));
    node.tasks.keep(resilum_tasks::watch(
        "coordinates: asking peers where they sit",
        ask_around(
            engine.clone(),
            router.clone(),
            node.coordinates.clone(),
            node.attachments.clone(),
            identity.clone(),
        ),
    ));
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
        let asking = whom::to_ask(&engine, &aspect, &attachments, round);
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
