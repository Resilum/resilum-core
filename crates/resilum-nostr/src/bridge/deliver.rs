//! Everything the bridge puts onto the mesh, off the loop: packing and
//! signing a message is the expensive part, and the poll has to stay
//! responsive while it happens.

use std::sync::Arc;
use std::time::Instant;

use data_encoding::HEXLOWER;
use serde_json::json;

use super::from_mesh::{Verdicts, ack_json};
use super::schema::SCHEMA_ACK;
use super::state::{self, State};
use super::tie::Tie;
use super::to_mesh::{Method, send_request};
use crate::queue::{Entry, Handoff};

/// What a failed submission was carrying, for the log line that says so.
#[derive(Clone, Copy)]
enum Carrying {
    Event,
    Acknowledgement,
}

impl Carrying {
    fn what(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Acknowledgement => "acknowledgement",
        }
    }
}

const DIRECT_TRIES_BEFORE_MAILBOX: u32 = 2;

fn on_schedule(handoff: Handoff) -> Option<(Method, Handoff)> {
    match handoff {
        Handoff::Direct { tries } if tries < DIRECT_TRIES_BEFORE_MAILBOX => Some((
            Method::Direct,
            Handoff::Direct {
                tries: tries.saturating_add(1),
            },
        )),
        Handoff::Direct { .. } => Some((Method::Propagated, Handoff::LeftWithPropagationNode)),
        Handoff::LeftWithPropagationNode => None,
    }
}

pub(super) fn inbound(state: &Arc<State>, entry: Entry) -> bool {
    let Some((method, handoff)) = on_schedule(entry.handoff) else {
        return false;
    };
    state
        .queue
        .set_handoff(&entry.event_id, &entry.subscriber, handoff);
    send(state, &entry, method);
    true
}

pub(super) fn to_a_device_back_on_the_mesh(
    state: &Arc<State>,
    address: &[u8; 16],
    moment: Instant,
) {
    let on_the_mesh = state.awaiting_report();
    for entry in state.queue.owed_to(address, state::now()) {
        let tie = Tie::from(&entry);
        if on_the_mesh.contains(&tie) || !state.retry.due(tie, moment) {
            continue;
        }
        state.retry.attempted(tie, moment);
        send(state, &entry, Method::Direct);
    }
}

/// The tie is recorded inside the submission, on a blocking thread: the LXMF
/// message id it is keyed by does not exist until the message is packed, so
/// it is not yet visible when this returns.
fn send(state: &Arc<State>, entry: &Entry, method: Method) {
    let tie = Tie::from(entry);
    submit(
        state,
        send_request(entry, method),
        Some(tie),
        Carrying::Event,
    );
}

pub(super) fn ack(state: &Arc<State>, dest: [u8; 16], event_id: &str, verdicts: &Verdicts) {
    let json = json!({
        "destination": HEXLOWER.encode(&dest),
        "method": "direct",
        "fields": {
            "custom_type": SCHEMA_ACK,
            "custom_data": ack_json(event_id, verdicts),
        }
    })
    .to_string();
    submit(state, json, None, Carrying::Acknowledgement);
}

fn submit(state: &Arc<State>, json: String, tie: Option<Tie>, carrying: Carrying) {
    let state = Arc::clone(state);
    tokio::task::spawn_blocking(move || {
        if let Err(error) = state.submit(&json, tie) {
            let what = carrying.what();
            tracing::warn!(%error, what, "the bridge could not put a message onto the mesh");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(rungs: usize) -> Vec<Option<(&'static str, Handoff)>> {
        let mut handoff = Handoff::default();
        (0..rungs)
            .map(|_| {
                let (method, next) = on_schedule(handoff)?;
                handoff = next;
                Some((method.token(), handoff))
            })
            .collect()
    }

    #[test]
    fn two_direct_tries_are_followed_by_the_mailbox_and_then_the_schedule_stops() {
        assert_eq!(
            ladder(4),
            vec![
                Some(("direct", Handoff::Direct { tries: 1 })),
                Some(("direct", Handoff::Direct { tries: 2 })),
                Some(("propagated", Handoff::LeftWithPropagationNode)),
                None,
            ]
        );
    }
}
