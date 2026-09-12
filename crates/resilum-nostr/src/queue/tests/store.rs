//! What a restart must still find on disk, and what it must refuse to guess.

use super::*;

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

/// A direction restored the wrong way round sends a subscriber's inbound
/// event back out to the public relays.
#[test]
fn a_queued_event_outlives_the_process_that_queued_it_field_for_field() {
    let dir = scratch();
    let path = dir.path().join("queue");
    let mut outbound = entry(2);
    outbound.direction = Direction::Outbound;
    outbound.lxmf = [0x5a; 16];
    outbound.subscriber = [0x6b; 32];
    outbound.event_json = r#"{"kind":1059,"content":"held"}"#.into();
    outbound.queued_at = 123;

    let queue = Queue::open(path.clone(), Duration::from_secs(600), 10).expect("opens");
    hold(&queue, entry(1));
    hold(&queue, outbound.clone());
    drop(queue);

    let reopened = Queue::open(path, Duration::from_secs(600), 10).expect("reopens");
    let due = reopened.due(200);
    assert_eq!(due.len(), 2);
    let restored = due
        .iter()
        .find(|e| e.event_id == outbound.event_id)
        .expect("the outbound entry came back");
    assert_eq!(restored.direction, Direction::Outbound);
    assert_eq!(restored.lxmf, outbound.lxmf);
    assert_eq!(restored.subscriber, outbound.subscriber);
    assert_eq!(restored.event_json, outbound.event_json);
    assert_eq!(restored.queued_at, outbound.queued_at);
    assert_eq!(
        due.iter()
            .find(|e| e.event_id == [1u8; 32])
            .expect("its sibling too")
            .direction,
        Direction::Inbound,
        "the two directions do not encode to the same string"
    );
}

#[test]
fn how_far_the_ladder_got_outlives_the_process() {
    let dir = scratch();
    let path = dir.path().join("queue");
    let deposited = entry(1);
    let trying = entry(2);

    let queue = Queue::open(path.clone(), Duration::from_secs(600), 10).expect("opens");
    hold(&queue, deposited.clone());
    hold(&queue, trying.clone());
    queue.set_handoff(
        &deposited.event_id,
        &deposited.subscriber,
        Handoff::LeftWithPropagationNode,
    );
    queue.set_handoff(
        &trying.event_id,
        &trying.subscriber,
        Handoff::Direct { tries: 1 },
    );
    drop(queue);

    let due = Queue::open(path, Duration::from_secs(600), 10)
        .expect("reopens")
        .due(200);
    let handoff = |id: [u8; 32]| {
        due.iter()
            .find(|e| e.event_id == id)
            .expect("the entry came back")
            .handoff
    };
    assert_eq!(
        handoff(deposited.event_id),
        Handoff::LeftWithPropagationNode
    );
    assert_eq!(handoff(trying.event_id), Handoff::Direct { tries: 1 });
}

/// Without the removal surviving the restart, the resolved entry comes back
/// and is delivered to a subscriber already told it landed.
#[test]
fn a_resolved_entry_does_not_survive_a_reopen_but_its_sibling_does() {
    let dir = scratch();
    let path = dir.path().join("queue");

    let queue = Queue::open(path.clone(), Duration::from_secs(600), 10).expect("opens");
    hold(&queue, entry(1));
    hold(&queue, entry(2));
    queue.resolve(&[1u8; 32], &[1u8; 32]);
    drop(queue);

    let reopened = Queue::open(path, Duration::from_secs(600), 10).expect("reopens");
    let due = reopened.due(100);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].event_id, [2u8; 32]);
}

#[test]
fn a_queue_file_that_cannot_be_read_refuses_to_open() {
    let a_directory_where_the_file_belongs = scratch();

    assert!(
        Queue::open(
            a_directory_where_the_file_belongs.path().to_path_buf(),
            Duration::from_secs(600),
            10
        )
        .is_err()
    );
}

#[test]
fn a_queue_file_that_does_not_exist_yet_opens_empty() {
    let dir = scratch();

    let queue = Queue::open(dir.path().join("queue"), Duration::from_secs(600), 10).expect("opens");
    assert!(queue.due(100).is_empty());
}
