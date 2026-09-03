//! Asking a peer where it sits, and answering when asked.

use std::sync::Arc;
use std::time::{Duration, Instant};

use leviculum_std::api::{
    Destination, DestinationHash, DestinationType, Direction, Identity, LinkHandle, LinkId,
};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Coordinates, PeerId};
use crate::link::{Inbound, LinkMsg, LinkRouter, answered};

pub(crate) const APP_NAME: &str = "resilum";
pub(crate) const ASPECT: &str = "coordinates";
const ANSWER_WITHIN: Duration = Duration::from_secs(20);
const WHICHEVER_ROUTE_RNS_RACED_TO: super::LinkId = 0;

#[must_use]
pub fn destination(identity: Identity) -> Destination {
    Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        &[ASPECT],
    )
    .expect("IN/SINGLE destination with an identity is always valid")
}

pub async fn answer(
    engine: Arc<ReticulumNode>,
    mut arriving: UnboundedReceiver<Inbound>,
    coordinates: Arc<Coordinates>,
    now: fn() -> f64,
    nursery: Arc<crate::nursery::Nursery>,
) {
    while let Some((link_id, _, from_link)) = arriving.recv().await {
        let handle = engine.link_handle(&link_id);
        nursery.keep(serve(
            engine.clone(),
            link_id,
            handle,
            from_link,
            coordinates.clone(),
            now(),
        ));
    }
}

async fn serve(
    engine: Arc<ReticulumNode>,
    link_id: LinkId,
    handle: LinkHandle,
    mut from_link: UnboundedReceiver<LinkMsg>,
    coordinates: Arc<Coordinates>,
    now: f64,
) {
    let theirs = answered(&mut from_link, ANSWER_WITHIN).await;
    let asker = engine
        .get_remote_identity(&link_id)
        .map(|identity| *identity.hash());
    if let (Some(theirs), Some(peer)) = (theirs, asker) {
        coordinates.heard(peer, theirs, now);
    }
    let ours = serde_json::to_vec(&coordinates.ours()).unwrap_or_default();
    let _ = handle.send(&ours).await;
}

pub async fn place(
    engine: &Arc<ReticulumNode>,
    router: &Arc<LinkRouter>,
    coordinates: &Arc<Coordinates>,
    us: &Identity,
    peer: PeerId,
    at: DestinationHash,
    now: f64,
) -> bool {
    let asking = data_encoding::HEXLOWER.encode(&peer);
    let Some((mut handle, link_id, mut from_link)) = crate::link::dial(engine, router, &at).await
    else {
        tracing::debug!(peer = %asking, "no link to ask a peer over");
        return false;
    };
    if let Err(error) = engine.identify_link(&link_id, us).await {
        tracing::debug!(peer = %asking, %error, "a peer was asked without saying who was asking");
    }
    let ours = serde_json::to_vec(&coordinates.ours()).unwrap_or_default();
    let asked_at = Instant::now();
    let theirs = match handle.send(&ours).await {
        Ok(()) => answered(&mut from_link, ANSWER_WITHIN).await,
        Err(error) => {
            tracing::debug!(peer = %asking, %error, "the ask never went out");
            None
        }
    };
    let rtt = least_round_trip_of(engine, &link_id).unwrap_or_else(|| asked_at.elapsed());
    router.detach(&link_id);
    let _ = handle.close().await;
    let Some(theirs) = theirs else {
        tracing::debug!(peer = %asking, "a peer was asked and said nothing");
        return false;
    };
    let believed = coordinates.believe(peer, WHICHEVER_ROUTE_RNS_RACED_TO, rtt, theirs, now);
    tracing::debug!(
        peer = %asking,
        rtt_ms = rtt.as_millis(),
        believed,
        theirs = %format_args!("{theirs:?}"),
        "a peer said where it sits"
    );
    believed
}

fn least_round_trip_of(engine: &Arc<ReticulumNode>, link_id: &LinkId) -> Option<Duration> {
    let least = engine.link_stats(link_id)?.min_rtt_ms()?;
    Some(Duration::from_millis(least))
}
