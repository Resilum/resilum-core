//! What becomes of an event once the relays have all answered — or once it
//! has waited long enough for them.

use super::{Publication, Publishing};
use crate::bridge::from_mesh::Verdicts;

mod queued;

pub(super) fn settle(bridge: &Publishing<'_>, event_id: &str, mut round: Publication) {
    let verdicts = Verdicts {
        accepted: round.accepted,
        rejected: round.rejected,
        reason: std::mem::take(&mut round.reason),
    };
    let waiting = queued::record(bridge, &round);
    for dest in answering(&round.reply_to, waiting) {
        (bridge.ack)(dest, event_id, &verdicts);
    }
}

fn answering(reply_to: &[[u8; 16]], waiting: Option<[u8; 16]>) -> Vec<[u8; 16]> {
    let mut answering = reply_to.to_vec();
    if let Some(waiting) = waiting
        && !answering.contains(&waiting)
    {
        answering.push(waiting);
    }
    answering
}
