//! What the re-queue leaves in the router's outbound queue.

use leviculum_lxmf::DeliveryMethod;

use super::{Harness, STAMP_COST, method_of, only_event, requeue, submit_direct};

#[test]
fn a_direct_message_requeued_as_propagated_is_queued_as_propagated() {
    let mut harness = Harness::new("requeue-method", STAMP_COST);
    let message_id = submit_direct(&mut harness);
    assert_eq!(
        method_of(&harness, &message_id),
        Some(DeliveryMethod::Direct),
        "setup did not queue a direct message",
    );

    requeue(&mut harness, message_id, DeliveryMethod::Propagated);

    assert_eq!(
        method_of(&harness, &message_id),
        Some(DeliveryMethod::Propagated),
    );
}

/// The whole point of the call: a caller escalating a message goes on addressing
/// it, and every delivery event for it, by the id its first send returned.
#[test]
fn the_message_id_is_unchanged_by_the_requeue() {
    let mut harness = Harness::new("requeue-id", STAMP_COST);
    let message_id = submit_direct(&mut harness);

    requeue(&mut harness, message_id, DeliveryMethod::Propagated);

    let queued: Vec<[u8; 32]> = harness.ready.router.outbound().keys().copied().collect();
    assert_eq!(queued, vec![message_id]);
    let entry_id = harness
        .ready
        .router
        .outbound()
        .get(&message_id)
        .map(|entry| entry.message().message_id);
    assert_eq!(entry_id, Some(message_id));
}

/// The re-queue cancels first, so a method the router will not take
/// would otherwise destroy a perfectly good queued message. `propagated` with
/// no propagation node selected is the reachable case, and the likely one: it
/// is exactly what a caller escalating a message asks for.
#[test]
fn a_refused_requeue_leaves_the_message_queued_under_its_old_method() {
    let mut harness = Harness::new("requeue-refused", STAMP_COST);
    let message_id = submit_direct(&mut harness);
    let _cleared = harness
        .ready
        .router
        .set_outbound_propagation_node(&mut harness.core, None)
        .expect("clear the selection");

    requeue(&mut harness, message_id, DeliveryMethod::Propagated);

    assert_eq!(
        method_of(&harness, &message_id),
        Some(DeliveryMethod::Direct)
    );
    let event = only_event(&harness, "requeue_refused");
    assert_eq!(event["reason"], "propagation_node_unavailable");
    assert_eq!(event["retry"], "requeue");
}
