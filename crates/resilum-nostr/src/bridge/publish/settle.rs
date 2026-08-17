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
    queued::record(bridge, &round);
    for dest in &round.reply_to {
        (bridge.ack)(*dest, event_id, &verdicts);
    }
}
