//! Everything the bridge puts onto the mesh, off the loop: packing and
//! signing a message is the expensive part, and the poll has to stay
//! responsive while it happens.

use std::sync::Arc;

use data_encoding::HEXLOWER;
use serde_json::json;

use super::from_mesh::{Verdicts, ack_json};
use super::schema::SCHEMA_ACK;
use super::state::State;
use super::tie::Tie;
use super::to_mesh::send_request;
use crate::queue::Entry;

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

/// The tie is recorded inside the submission, on a blocking thread: the LXMF
/// message id it is keyed by does not exist until the message is packed, so
/// it is not yet visible when this returns.
pub(super) fn event(state: &Arc<State>, entry: Entry) {
    let tie = Tie::from(&entry);
    submit(state, send_request(&entry), Some(tie), Carrying::Event);
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
