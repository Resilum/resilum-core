use data_encoding::HEXLOWER;
use leviculum_lxmf::router::{MessageState, RouterEvent};
use serde_json::Value;

use super::event_to_json;

#[test]
fn the_giveup_path_reports_attempts_exhausted() {
    let mid = [9u8; 32];
    let event = RouterEvent::MessageState {
        message_id: mid,
        state: MessageState::Failed,
    };

    let v: Value = serde_json::from_str(&event_to_json(&event, 0.0).unwrap()).unwrap();

    assert_eq!(v["message_id"], HEXLOWER.encode(&mid));
    assert_eq!(v["state"], "failed");
    assert_eq!(v["reason"], "attempts_exhausted");
    assert_eq!(v["retry"], "resubmit");
}

/// `reason`/`retry` only make sense on `failed` — carrying them on `delivered`
/// or `sent` would imply those states can fail too.
#[test]
fn only_failed_carries_a_reason_and_a_retry_token() {
    for state in [
        MessageState::Generating,
        MessageState::Outbound,
        MessageState::Sending,
        MessageState::Sent,
        MessageState::AwaitingCollection,
        MessageState::Delivered,
        MessageState::Rejected,
        MessageState::Cancelled,
    ] {
        let event = RouterEvent::MessageState {
            message_id: [1u8; 32],
            state,
        };

        let v: Value = serde_json::from_str(&event_to_json(&event, 0.0).unwrap()).unwrap();

        assert!(
            v.get("reason").is_none(),
            "{state:?} should carry no reason"
        );
        assert!(v.get("retry").is_none(), "{state:?} should carry no retry");
    }
}
