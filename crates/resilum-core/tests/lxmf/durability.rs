//! The outbound queue outliving the process that made it.

use data_encoding::BASE64;
use serde_json::json;

use crate::common::{next_event, start, submit, temp_dir, wait_for};

/// Nobody is listening on this address, so the message stays queued and keeps
/// being retried — which is the state that has to survive a restart.
const UNREACHABLE: &str = "00112233445566778899aabbccddeeff";

fn queued_message() -> String {
    json!({
        "dest": UNREACHABLE,
        "method": "direct",
        "content_b64": BASE64.encode(b"outlives the process"),
    })
    .to_string()
}

/// On a phone the OS kills the app whenever it likes. If the outbound queue
/// dies with the process, "send" is a coin flip.
///
/// The restored queue is read back through the router's own duplicate check:
/// submitting the same message again is refused only if the router still has
/// the first one. That is both the fastest signal available and an honest one —
/// waiting for the retry itself would be waiting out a delivery timeout, which
/// says nothing about whether the queue was restored.
#[test]
fn an_undelivered_message_survives_a_restart() {
    let dir = temp_dir("durable");
    let checkpoint = dir.join("lxmf_state");

    let mut node = start("durable", &dir, None, Vec::new());
    let message_id = submit(&node, &queued_message());
    wait_for("the checkpoint to be written", || {
        checkpoint.metadata().ok().filter(|meta| meta.len() > 0)
    });
    node.stop().expect("stop");

    // Same storage directory, so the same identity and the same checkpoint.
    let restarted = start("durable", &dir, None, Vec::new());
    let resubmitted = submit(&restarted, &queued_message());
    assert_eq!(resubmitted, message_id, "same message, same id");

    let event = next_event(&restarted, "delivery", "the duplicate to be refused");
    assert_eq!(event["state"], "failed");
    assert_eq!(event["message_id"], message_id);
}
