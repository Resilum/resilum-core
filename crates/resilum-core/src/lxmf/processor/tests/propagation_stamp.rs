//! The propagation-stamp path: the router asks, the stamp thread answers, the
//! router takes the answer — or has moved on, which is not a failure.

use super::harness::{Harness, STAMP_COST};
use super::scenario::{answer, ask, failures, grind, submit_propagated, wants_stamp};
use crate::lxmf::stamp::Job;

#[test]
fn a_propagation_stamp_request_reaches_the_stamp_thread() {
    let mut harness = Harness::new("reaches", STAMP_COST);
    let message_id = submit_propagated(&mut harness);

    let request = ask(&mut harness);

    assert_eq!(request.message_id, message_id);
    assert_eq!(u64::from(request.target_cost), STAMP_COST);
}

#[test]
fn a_mined_stamp_is_applied_to_the_waiting_message() {
    let mut harness = Harness::new("applied", STAMP_COST);
    let message_id = submit_propagated(&mut harness);
    let request = ask(&mut harness);
    assert!(
        wants_stamp(&harness, &message_id),
        "the message should be waiting on a stamp",
    );

    answer(&mut harness, request, Ok(grind(&request)));

    assert_eq!(harness.ready.router.outbound().len(), 1, "message dropped");
    assert!(
        !wants_stamp(&harness, &message_id),
        "the stamp was ground but never reached the router",
    );
}

/// The router re-asks while a stamp is being ground, so an answer can arrive
/// for an entry that has one already. Ordinary, and not something the caller
/// should ever hear about.
#[test]
fn a_stale_result_is_dropped_without_telling_the_caller() {
    let mut harness = Harness::new("stale", STAMP_COST);
    let message_id = submit_propagated(&mut harness);
    let request = ask(&mut harness);
    let stamp = grind(&request);
    answer(&mut harness, request, Ok(stamp));

    answer(&mut harness, request, Ok(stamp));

    assert_eq!(harness.ready.router.outbound().len(), 1, "message dropped");
    assert!(
        !wants_stamp(&harness, &message_id),
        "the applied stamp was undone",
    );
    assert_eq!(failures(&harness), Vec::<String>::new());
}

#[test]
fn a_repeated_ask_does_not_queue_a_second_grind() {
    let mut harness = Harness::new("repeat", STAMP_COST);
    submit_propagated(&mut harness);
    let request = ask(&mut harness);

    harness.ask_again(request);

    assert!(
        harness.jobs.try_recv().is_err(),
        "the same grind was queued twice",
    );
    answer(&mut harness, request, Err("interrupted".into()));
    harness.ask_again(request);
    assert!(
        matches!(harness.jobs.try_recv(), Ok(Job::Propagation(_))),
        "a message off the thread must be grindable again",
    );
}
