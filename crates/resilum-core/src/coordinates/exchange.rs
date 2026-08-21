//! Asking a peer where it sits, and answering when asked.

use std::sync::Arc;
use std::time::{Duration, Instant};

use leviculum_std::api::{
    Destination, DestinationHash, DestinationType, Direction, Identity, LinkHandle, LinkId,
};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

use super::{Claimed, Coordinates, PeerId};
use crate::link::{Inbound, LinkMsg, LinkRouter};

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
) {
    while let Some((link_id, _, from_link)) = arriving.recv().await {
        let handle = engine.link_handle(&link_id);
        let asker = engine
            .get_remote_identity(&link_id)
            .map(|identity| *identity.hash());
        tokio::spawn(serve(handle, from_link, coordinates.clone(), asker, now()));
    }
}

async fn serve(
    mut handle: LinkHandle,
    mut from_link: UnboundedReceiver<LinkMsg>,
    coordinates: Arc<Coordinates>,
    asker: Option<PeerId>,
    now: f64,
) {
    if let (Some(theirs), Some(peer)) = (read(&mut from_link).await, asker) {
        coordinates.heard(peer, theirs, now);
    }
    let ours = serde_json::to_vec(&coordinates.ours()).unwrap_or_default();
    let _ = handle.send(&ours).await;
    let _ = handle.close().await;
}

pub async fn place(
    engine: &Arc<ReticulumNode>,
    router: &Arc<LinkRouter>,
    coordinates: &Arc<Coordinates>,
    peer: PeerId,
    at: DestinationHash,
    now: f64,
) -> bool {
    let Some((mut handle, link_id, mut from_link)) = crate::link::dial(engine, router, &at).await
    else {
        return false;
    };
    let ours = serde_json::to_vec(&coordinates.ours()).unwrap_or_default();
    let asked_at = Instant::now();
    let theirs = match handle.send(&ours).await {
        Ok(()) => read(&mut from_link).await,
        Err(_) => None,
    };
    let rtt = least_round_trip_of(engine, &link_id).unwrap_or_else(|| asked_at.elapsed());
    router.detach(&link_id);
    let _ = handle.close().await;
    match theirs {
        Some(theirs) => {
            tracing::debug!(
                peer = %data_encoding::HEXLOWER.encode(&peer),
                rtt_ms = rtt.as_millis(),
                theirs = %format_args!("{theirs:?}"),
                "a peer said where it sits"
            );
            coordinates.believe(peer, WHICHEVER_ROUTE_RNS_RACED_TO, rtt, theirs, now)
        }
        None => false,
    }
}

fn least_round_trip_of(engine: &Arc<ReticulumNode>, link_id: &LinkId) -> Option<Duration> {
    let least = engine.link_stats(link_id)?.min_rtt_ms()?;
    Some(Duration::from_millis(least))
}

async fn read(from_link: &mut UnboundedReceiver<LinkMsg>) -> Option<Claimed> {
    loop {
        match timeout(ANSWER_WITHIN, from_link.recv()).await {
            Ok(Some(LinkMsg::Data(bytes))) => return serde_json::from_slice(&bytes).ok(),
            Ok(Some(LinkMsg::Established)) => continue,
            _ => return None,
        }
    }
}
