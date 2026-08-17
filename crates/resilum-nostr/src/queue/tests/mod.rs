use super::*;

mod store;

fn hold(queue: &Queue, entry: Entry) {
    assert_eq!(queue.push(entry), Queued::Held, "the queue took the setup");
}

fn entry(n: u8) -> Entry {
    Entry {
        direction: Direction::Inbound,
        subscriber: [1u8; 32],
        lxmf: [2u8; 16],
        event_id: [n; 32],
        event_json: "{}".into(),
        queued_at: 100,
    }
}

#[test]
fn a_delivered_entry_stops_being_due() {
    let queue = Queue::ephemeral(Duration::from_secs(600), 10);
    hold(&queue, entry(1));
    hold(&queue, entry(2));
    queue.resolve(&[1u8; 32], &[1u8; 32]);

    let due = queue.due(100);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].event_id, [2u8; 32]);
}

#[test]
fn a_subscriber_at_the_ceiling_refuses_new_entries() {
    let queue = Queue::ephemeral(Duration::from_secs(600), 2);

    assert_eq!(queue.push(entry(1)), Queued::Held);
    assert_eq!(queue.push(entry(2)), Queued::Held);
    assert_eq!(queue.push(entry(3)), Queued::AtCeiling);
    assert_eq!(queue.due(100).len(), 2);
}

#[test]
fn an_entry_past_retention_is_dropped() {
    let queue = Queue::ephemeral(Duration::from_secs(60), 10);
    hold(&queue, entry(1));

    assert_eq!(queue.expire(1000), 1);
    assert!(queue.due(1000).is_empty());
}

#[test]
fn an_entry_past_retention_does_not_hold_its_ceiling_slot() {
    let queue = Queue::ephemeral(Duration::from_secs(60), 2);
    assert_eq!(queue.push(entry(1)), Queued::Held);
    assert_eq!(queue.push(entry(2)), Queued::Held);

    let mut arriving = entry(3);
    arriving.queued_at = 1000; // long past entries 1 and 2's 60s retention

    assert_eq!(queue.push(arriving), Queued::Held);
}

#[test]
fn an_entry_the_queue_already_holds_is_not_taken_again() {
    let queue = Queue::ephemeral(Duration::from_secs(600), 10);

    assert_eq!(queue.push(entry(1)), Queued::Held);
    assert_eq!(queue.push(entry(1)), Queued::AlreadyHeld);
    assert_eq!(queue.due(100).len(), 1);
}

#[test]
fn a_different_subscribers_copy_of_the_same_event_is_taken_too() {
    let queue = Queue::ephemeral(Duration::from_secs(600), 10);
    hold(&queue, entry(1));

    let mut theirs = entry(1);
    theirs.subscriber = [9u8; 32];

    assert_eq!(queue.push(theirs), Queued::Held);
}

#[test]
fn an_entry_past_retention_does_not_answer_for_a_re_offer() {
    let queue = Queue::ephemeral(Duration::from_secs(60), 10);
    hold(&queue, entry(1));

    let mut again = entry(1);
    again.queued_at = 1000;

    assert_eq!(queue.push(again), Queued::Held);
}
