use std::time::Duration;

use super::{before_trying_again, it_carried_nobody};

#[test]
fn a_group_carrying_someone_is_never_given_up() {
    assert!(!it_carried_nobody(1, Duration::from_secs(3_600)));
}

#[test]
fn a_group_is_given_time_before_it_counts_as_empty() {
    assert!(!it_carried_nobody(0, Duration::from_secs(10)));
    assert!(it_carried_nobody(0, Duration::from_secs(120)));
}

#[test]
fn each_refusal_waits_longer_than_the_last() {
    let waits: Vec<Duration> = (1..=4).map(before_trying_again).collect();

    for pair in waits.windows(2) {
        assert!(pair[1] > pair[0], "{:?} did not grow", waits);
    }
}

#[test]
fn the_wait_stops_growing_rather_than_running_away() {
    assert_eq!(before_trying_again(20), before_trying_again(30));
    assert!(before_trying_again(u32::MAX) <= Duration::from_secs(1_800));
}

#[test]
fn the_first_refusal_waits_the_shortest_time() {
    assert_eq!(before_trying_again(1), Duration::from_secs(60));
}
