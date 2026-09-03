use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{
    Destination, DestinationHash, DestinationType, Direction, Identity, LinkHandle, LinkId,
};
use leviculum_std::driver::ReticulumNode;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Facts, Field, WhatThePlatformKnows};
use crate::ble::links::PeerId;
use crate::link::{Inbound, LinkMsg, LinkRouter, answered};

const APP_NAME: &str = "resilum";
const ASPECTS: [&str; 2] = ["ble", "election"];
const ANSWER_WITHIN: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhatACandidateTells {
    pub facts: Facts,
    pub can_host_at_all: bool,
}

#[must_use]
pub fn destination(identity: Identity) -> Destination {
    Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        &ASPECTS,
    )
    .expect("IN/SINGLE destination with an identity is always valid")
}

#[must_use]
pub fn where_a_candidate_answers(peer: PeerId) -> DestinationHash {
    Destination::compute_destination_hash(
        &Destination::compute_name_hash(APP_NAME, &ASPECTS),
        &peer,
    )
}

#[must_use]
pub fn what_we_tell(known: &WhatThePlatformKnows, field: &Field) -> WhatACandidateTells {
    WhatACandidateTells {
        facts: known.read(field.how_many_we_hear()),
        can_host_at_all: known.can_host_at_all(),
    }
}

pub async fn answer(
    engine: Arc<ReticulumNode>,
    mut arriving: UnboundedReceiver<Inbound>,
    known: WhatThePlatformKnows,
    field: Field,
    nursery: Arc<crate::nursery::Nursery>,
) {
    while let Some((link_id, _, from_link)) = arriving.recv().await {
        let handle = engine.link_handle(&link_id);
        nursery.keep(serve(
            engine.clone(),
            link_id,
            handle,
            from_link,
            known.clone(),
            field.clone(),
        ));
    }
}

async fn serve(
    engine: Arc<ReticulumNode>,
    link_id: LinkId,
    handle: LinkHandle,
    mut from_link: UnboundedReceiver<LinkMsg>,
    known: WhatThePlatformKnows,
    field: Field,
) {
    let theirs = answered::<WhatACandidateTells>(&mut from_link, ANSWER_WITHIN).await;
    let asker = engine
        .get_remote_identity(&link_id)
        .map(|identity| *identity.hash());
    if let (Some(theirs), Some(peer)) = (theirs, asker) {
        field.told_us(peer, theirs.facts, theirs.can_host_at_all);
    }
    let ours = serde_json::to_vec(&what_we_tell(&known, &field)).unwrap_or_default();
    let _ = handle.send(&ours).await;
}

pub async fn ask(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    field: Field,
    us: Identity,
    peer: PeerId,
    ours: WhatACandidateTells,
) {
    let at = where_a_candidate_answers(peer);
    let Some((mut handle, link_id, mut from_link)) = crate::link::dial(&engine, &router, &at).await
    else {
        tracing::debug!(peer = %crate::hex::encode(peer.iter()), "no link to ask a candidate over");
        return;
    };
    if let Err(error) = engine.identify_link(&link_id, &us).await {
        tracing::debug!(%error, "a candidate was asked without saying who was asking");
    }
    let told = match handle
        .send(&serde_json::to_vec(&ours).unwrap_or_default())
        .await
    {
        Ok(()) => answered::<WhatACandidateTells>(&mut from_link, ANSWER_WITHIN).await,
        Err(error) => {
            tracing::debug!(%error, "the ask never went out");
            None
        }
    };
    router.detach(&link_id);
    let _ = handle.close().await;
    if let Some(theirs) = told {
        field.told_us(peer, theirs.facts, theirs.can_host_at_all);
    }
}
