use super::*;

fn tie() -> Tie {
    Tie {
        event_id: [1u8; 32],
        subscriber: [7u8; 32],
    }
}

#[test]
fn every_attempt_waits_longer_than_the_one_before() {
    let schedule = Schedule::default();
    let start = Instant::now();

    assert!(schedule.due(tie(), start));
    schedule.attempted(tie(), start);
    assert!(!schedule.due(tie(), start + Duration::from_secs(59)));

    let second = start + Duration::from_secs(61);
    assert!(schedule.due(tie(), second));
    schedule.attempted(tie(), second);
    assert!(!schedule.due(tie(), second + Duration::from_secs(119)));
    assert!(schedule.due(tie(), second + Duration::from_secs(121)));
}

#[test]
fn an_announce_sends_at_once_however_far_the_doubling_has_walked() {
    let schedule = Schedule::default();
    let start = Instant::now();
    for _ in 0..5 {
        schedule.attempted(tie(), start);
    }
    let ten_minutes = start + Duration::from_secs(600);
    assert!(!schedule.due(tie(), ten_minutes), "the wait is sixteen");

    assert!(schedule.reachable_now(tie(), ten_minutes));

    assert!(schedule.due(tie(), ten_minutes));
}

#[test]
fn a_device_announcing_every_few_seconds_is_not_sent_to_every_few_seconds() {
    let schedule = Schedule::default();
    let start = Instant::now();
    schedule.attempted(tie(), start);

    assert!(!schedule.reachable_now(tie(), start + Duration::from_secs(5)));
    assert!(!schedule.reachable_now(tie(), start + Duration::from_secs(59)));
    assert!(schedule.reachable_now(tie(), start + Duration::from_secs(61)));
}

#[test]
fn an_announce_for_an_entry_never_attempted_sends_at_once() {
    let schedule = Schedule::default();

    assert!(schedule.reachable_now(tie(), Instant::now()));
}

#[test]
fn the_wait_stops_growing_at_an_hour() {
    assert_eq!(backoff(20), CAP);
}

#[test]
fn an_entry_that_left_the_queue_is_forgotten() {
    let schedule = Schedule::default();
    let start = Instant::now();
    schedule.attempted(tie(), start);

    schedule.keep_only(&[]);

    assert!(schedule.due(tie(), start));
}
