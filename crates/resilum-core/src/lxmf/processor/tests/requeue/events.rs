//! What the caller hears when a message is re-queued.

use data_encoding::HEXLOWER;
use leviculum_core::transport::TickOutput;
use leviculum_lxmf::DeliveryMethod;

use super::{ABSENT, Harness, STAMP_COST, events, only_event, requeue, submit_direct};

#[test]
fn the_requeue_reports_no_cancelled_to_the_caller() {
    let mut harness = Harness::new("requeue-quiet", STAMP_COST);
    let message_id = submit_direct(&mut harness);
    let _drained = events(&harness);

    requeue(&mut harness, message_id, DeliveryMethod::Propagated);

    let states: Vec<String> = events(&harness)
        .iter()
        .filter_map(|event| event["state"].as_str().map(str::to_owned))
        .collect();
    assert!(
        !states.iter().any(|state| state == "cancelled"),
        "{states:?}"
    );
}

#[test]
fn an_ordinary_cancellation_still_reaches_the_caller() {
    let mut harness = Harness::new("cancel-heard", STAMP_COST);
    let message_id = submit_direct(&mut harness);
    let _drained = events(&harness);

    let cancelled = harness
        .ready
        .router
        .cancel(&mut harness.core, &message_id)
        .expect("cancel the queued message");
    let mut out = TickOutput::empty();
    harness
        .processor
        .absorb(&mut harness.ready, &mut harness.core, cancelled, &mut out);

    let event = only_event(&harness, "delivery");
    assert_eq!(event["state"], "cancelled");
    assert_eq!(event["message_id"], HEXLOWER.encode(&message_id));
}

/// Not a `delivery` event: the router no longer holding the id means it
/// already reached a terminal state the caller was told about, and a `failed`
/// over a `delivered` would be worse than saying nothing.
#[test]
fn requeuing_an_id_the_router_does_not_hold_is_refused_to_the_caller() {
    let mut harness = Harness::new("requeue-absent-event", STAMP_COST);

    requeue(&mut harness, ABSENT, DeliveryMethod::Propagated);

    let event = only_event(&harness, "requeue_refused");
    assert_eq!(event["message_id"], HEXLOWER.encode(&ABSENT));
    assert_eq!(event["reason"], "not_found");
    assert_eq!(event["retry"], "never");
}
