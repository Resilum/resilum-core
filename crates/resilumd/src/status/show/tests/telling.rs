use std::time::Duration;

use resilum_core::status::Link;

use super::super::render::all_of_it;
use super::{PLAINLY, a_carrier, a_node, without_colour};

#[test]
fn an_interface_that_is_down_is_not_shown_as_up() {
    let mut status = a_node();
    status.interfaces = vec![a_carrier("anchor", false)];

    let shown = without_colour(&all_of_it(&status, Some(Duration::from_secs(1)), &PLAINLY));

    let row = shown
        .lines()
        .find(|line| line.contains("anchor"))
        .unwrap_or_default();
    assert!(row.contains("down"), "{shown}");
}

#[test]
fn a_node_that_stopped_writing_reads_as_silent_even_though_it_claimed_to_be_running() {
    let running = a_node();
    assert!(running.running);

    let stale = without_colour(&all_of_it(
        &running,
        Some(Duration::from_secs(600)),
        &PLAINLY,
    ));
    let fresh = without_colour(&all_of_it(&running, Some(Duration::from_secs(2)), &PLAINLY));

    assert!(stale.contains("silent for 10m"), "{stale}");
    assert!(!stale.contains("running"), "{stale}");
    assert!(fresh.contains("running"), "{fresh}");
}

#[test]
fn a_link_without_a_measured_round_trip_says_unknown_rather_than_zero() {
    let mut status = a_node();
    status.links = vec![Link {
        identity_hash: "1122334455667788".to_owned(),
        transport: "tcp".to_owned(),
        interface_name: None,
        estimated_rtt_ms: None,
    }];

    let shown = all_of_it(&status, Some(Duration::from_secs(1)), &PLAINLY);

    assert!(shown.contains("unknown"), "{shown}");
}
