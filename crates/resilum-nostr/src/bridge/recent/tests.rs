use super::*;

const SUBSCRIBER: [u8; 32] = [0xab; 32];

fn event(byte: u8) -> [u8; 32] {
    [byte; 32]
}

#[test]
fn an_event_the_window_still_covers_is_remembered_past_the_old_ceiling() {
    let recent = Recent::default();
    let first = event(1);
    recent.remember(&SUBSCRIBER, first, 1_000);
    for i in 0..200u32 {
        recent.remember(&SUBSCRIBER, [i as u8; 32], 1_000 + i64::from(i));
    }

    assert!(recent.seen(&SUBSCRIBER, &first));
}

#[test]
fn an_event_older_than_the_window_is_forgotten_because_no_req_can_ask_for_it() {
    let recent = Recent::default();
    let ancient = event(1);
    recent.remember(&SUBSCRIBER, ancient, 1_000);

    recent.remember(&SUBSCRIBER, event(2), 1_000 + NIP59_BACKDATE + 1);

    assert!(!recent.seen(&SUBSCRIBER, &ancient));
    assert!(recent.seen(&SUBSCRIBER, &event(2)));
}

#[test]
fn an_event_arriving_out_of_order_does_not_pull_the_window_backwards() {
    let recent = Recent::default();
    recent.remember(&SUBSCRIBER, event(1), 1_000 + NIP59_BACKDATE);
    let backdated = event(2);

    recent.remember(&SUBSCRIBER, backdated, 1_001);

    assert!(recent.seen(&SUBSCRIBER, &backdated));
    assert!(recent.seen(&SUBSCRIBER, &event(1)));
}
