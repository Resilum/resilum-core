//! Driving one propagated send from submission to a stamp result.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::{CooperativeStamper, PropagationStampRequest};
use serde_json::{Value, json};

use super::harness::Harness;
use super::node::known_peer;
use crate::lxmf::handle::Command;
use crate::lxmf::stamp::{Job, Outcome};

/// Queue a message for a peer that is only reachable through a propagation
/// node.
pub(super) fn submit_propagated(harness: &mut Harness) -> [u8; 32] {
    let request = json!({
        "destination": HEXLOWER.encode(&known_peer(&mut harness.core)),
        "method": "propagated",
        "content_b64": BASE64.encode(b"stored and forwarded"),
    })
    .to_string();
    let source_hash = crate::identity::lxmf_address(&harness.identity);
    let message = crate::lxmf::send::build_message(&request, &harness.identity, source_hash, 1.0)
        .expect("message");
    let message_id = message.message_id;
    harness
        .commands
        .send(Command::Send(Box::new(message)))
        .expect("submit");
    harness.pump();
    message_id
}

/// What the router asks the stamp thread for on its next pass.
pub(super) fn ask(harness: &mut Harness) -> PropagationStampRequest {
    harness.tick();
    match harness.jobs.try_recv() {
        Ok(Job::Propagation(request)) => request,
        Ok(Job::Delivery(_)) => panic!("the recipient's stamp, not the node's"),
        Err(e) => panic!("no propagation grind reached the stamp thread: {e}"),
    }
}

/// Hand the answer back the way the stamp thread does.
pub(super) fn answer(
    harness: &mut Harness,
    request: PropagationStampRequest,
    stamp: Result<[u8; 32], String>,
) {
    harness
        .commands
        .send(Command::Stamp(Outcome::Propagation { request, stamp }))
        .expect("answer");
    harness.pump();
}

pub(super) fn grind(request: &PropagationStampRequest) -> [u8; 32] {
    let mut stamper = CooperativeStamper::cooperative(rand_core::OsRng);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(request.generate_with(&mut stamper))
        .expect("ground stamp")
}

pub(super) fn wants_stamp(harness: &Harness, message_id: &[u8; 32]) -> bool {
    harness
        .ready
        .router
        .outbound_propagation_stamp_request(message_id)
        .is_some()
}

/// Every failure verdict a caller would read.
pub(super) fn failures(harness: &Harness) -> Vec<String> {
    let mut failures = Vec::new();
    while let Some(json) = harness.handle.next_event() {
        let event: Value = serde_json::from_str(&json).expect("event json");
        if event["state"] == "failed" {
            failures.push(json);
        }
    }
    failures
}
