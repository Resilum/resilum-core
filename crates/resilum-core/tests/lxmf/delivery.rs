//! Getting a message from one node to another.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_std::DestinationHash;
use serde_json::json;

use crate::common::{free_port, next_event, start, submit, temp_dir, wait_for};

/// A message the router refuses is refused *after* `submit` has returned, so
/// the only way the app can hear about it is a delivery update. Without one it
/// would sit in the UI as "sending" forever. Propagated delivery is the
/// reachable case: no propagation node has announced itself on a lone node, so
/// there is no mailbox to hand it to.
#[test]
fn a_refused_message_comes_back_as_a_failed_delivery() {
    let node = start("reject", &temp_dir("reject"), None, Vec::new());

    let message_id = submit(
        &node,
        &json!({
            "dest": "00112233445566778899aabbccddeeff",
            "method": "propagated",
            "content_b64": BASE64.encode(b"nowhere to go"),
        })
        .to_string(),
    );

    let event = next_event(&node, "delivery", "the failure to be reported");
    assert_eq!(event["state"], "failed");
    assert_eq!(event["message_id"], message_id);
}

#[test]
fn a_message_reaches_the_peer_that_announced_its_delivery_address() {
    let port = free_port();
    let receiver = start("rx", &temp_dir("rx"), Some(port), Vec::new());
    let sender = start(
        "tx",
        &temp_dir("tx"),
        None,
        vec![format!("127.0.0.1:{port}")],
    );

    // The sender cannot encrypt to a destination whose identity it has not
    // learned, and that only arrives with the receiver's delivery announce.
    let address = receiver.lxmf_address().expect("receiver address");
    let raw = HEXLOWER.decode(address.as_bytes()).expect("hex address");
    let dest = DestinationHash::new(<[u8; 16]>::try_from(raw.as_slice()).expect("16 bytes"));
    wait_for("the delivery announce to arrive", || {
        sender.engine()?.get_identity(&dest)
    });

    let body = b"hello over lxmf";
    submit(
        &sender,
        &json!({
            "dest": address,
            "method": "opportunistic",
            "content_b64": BASE64.encode(body),
        })
        .to_string(),
    );

    let event = next_event(&receiver, "message", "the message to arrive");
    assert_eq!(event["content_b64"], BASE64.encode(body));
    assert_eq!(
        event["source"],
        sender.lxmf_address().expect("sender address")
    );
}
