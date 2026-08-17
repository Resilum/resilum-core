use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use leviculum_lxmf::router::RouterError;
use serde_json::Value;

use super::report_enqueue_error;
use crate::lxmf::handle::channel;
use crate::lxmf::inbox::Inbox;

fn sink() -> (
    crate::lxmf::handle::LxmfHandle,
    crate::lxmf::handle::EventSink,
) {
    let inbox = Arc::new(Inbox::ephemeral());
    let (handle, _commands, events) = channel("aa".into(), Arc::new(AtomicBool::new(true)), inbox);
    (handle, events)
}

#[test]
fn a_duplicate_produces_no_event() {
    let (handle, mut events) = sink();

    report_enqueue_error(&mut events, &[1u8; 32], RouterError::Duplicate);

    assert_eq!(handle.next_event(), None);
}

/// Every other enqueue error still has to reach the caller as `failed`, or the
/// message stays "sending" forever — this is what stops the `Duplicate`
/// carve-out from silently swallowing a real failure too.
#[test]
fn any_other_error_still_produces_failed() {
    let (handle, mut events) = sink();

    report_enqueue_error(&mut events, &[2u8; 32], RouterError::QueueFull);

    let json = handle.next_event().expect("a failed event");
    let v: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["type"], "delivery");
    assert_eq!(v["state"], "failed");
    assert_eq!(v["reason"], "queue_full");
    assert_eq!(v["retry"], "resubmit");
}
