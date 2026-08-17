//! The outbound queue outliving the process that made it.

use data_encoding::{BASE64, HEXLOWER};
use serde_json::json;

use crate::common::{start, submit, temp_dir, wait_for};

/// Nobody is listening on this address, so the message stays queued and keeps
/// being retried — which is the state that has to survive a restart.
const UNREACHABLE: &str = "00112233445566778899aabbccddeeff";

fn queued_message() -> String {
    json!({
        "destination": UNREACHABLE,
        "method": "direct",
        "content_b64": BASE64.encode(b"outlives the process"),
    })
    .to_string()
}

/// A process can be killed at any moment, by an operator or by the OS. If the
/// outbound queue dies with it, "send" is a coin flip.
///
/// The restored queue is read straight off the router's own count of it, and
/// nothing is submitted to the restarted node: a queue that came back empty
/// leaves that count at zero for good, so no second path can satisfy the
/// assertion. Delivery events are no use here — the router reports the same
/// `failed`, under the same id, for the restored message and for a fresh copy
/// of it, and only the retry schedule tells the two apart.
#[test]
fn an_undelivered_message_survives_a_restart() {
    let dir = temp_dir("durable");
    let checkpoint = dir.join("lxmf_state");

    let mut node = start("durable", &dir, None, Vec::new());
    submit(&node, &queued_message());
    wait_for("the checkpoint to be written", || {
        checkpoint.metadata().ok().filter(|meta| meta.len() > 0)
    });
    node.stop().expect("stop");

    // Same storage directory, so the same identity and the same checkpoint.
    let restarted = start("durable", &dir, None, Vec::new());
    let depth = wait_for("the restored queue to be counted", || {
        let depth = restarted.lxmf()?.outbound_depth();
        (depth > 0).then_some(depth)
    });
    assert_eq!(depth, 1, "the one undelivered message, and only it");
}

/// A caller that lost its own record reconciles against the ids the router is
/// still carrying, and that only works if an id means the same thing on both
/// sides of a restart.
///
/// Nothing stores the id: the checkpoint holds the packed message, and the
/// restored id is recomputed from it on the way back in. So this is a claim
/// about a hash surviving a pack/unpack round trip, not about a field being
/// copied, and it is worth a test of its own rather than an assumption.
#[test]
fn a_queued_message_keeps_its_id_across_a_restart() {
    let dir = temp_dir("durable-id");
    let checkpoint = dir.join("lxmf_state");

    let mut node = start("durable-id", &dir, None, Vec::new());
    let submitted = submit(&node, &queued_message());
    wait_for("the checkpoint to be written", || {
        checkpoint.metadata().ok().filter(|meta| meta.len() > 0)
    });
    node.stop().expect("stop");

    let restarted = start("durable-id", &dir, None, Vec::new());
    let restored = wait_for("the restored queue to be published", || {
        let ids = restarted.lxmf()?.queued_ids();
        (!ids.is_empty()).then_some(ids)
    });

    let restored: Vec<String> = restored.iter().map(|id| HEXLOWER.encode(id)).collect();
    assert_eq!(restored, vec![submitted]);
}
