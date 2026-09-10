use std::time::Duration;

use resilum_core::status::Transport;

use super::super::render::all_of_it;
use super::super::units::{WIDEST_NAME_WE_SHOW, bytes, keeping_both_ends, name_column};
use super::{IN_SHORT, PLAINLY, a_carrier, a_link, a_node, without_colour};

#[test]
fn an_onion_name_too_wide_for_the_column_keeps_the_ends_that_identify_it() {
    let onion =
        "TorDiscovered[f43d2sqmkxyur7u7d64otl5x2s6ww7ccr7yoduln5qj3fmwu6tpufgad.onion]:4242";

    let shown = keeping_both_ends(onion, WIDEST_NAME_WE_SHOW);

    assert_eq!(shown.chars().count(), WIDEST_NAME_WE_SHOW);
    assert!(shown.starts_with("TorDiscovered[f43d"), "{shown}");
    assert!(shown.ends_with("]:4242"), "{shown}");
}

#[test]
fn a_name_that_already_fits_is_left_alone() {
    assert_eq!(
        keeping_both_ends("tcp_client_3", WIDEST_NAME_WE_SHOW),
        "tcp_client_3"
    );
}

#[test]
fn short_names_keep_the_table_narrow_and_one_long_one_does_not_widen_it_past_the_terminal() {
    let mut status = a_node();
    status.interfaces = vec![a_carrier("tcp_client_3", true)];
    assert_eq!(name_column(&status), "tcp_client_3".len());

    status.interfaces.push(a_carrier(&"x".repeat(200), true));
    assert_eq!(name_column(&status), WIDEST_NAME_WE_SHOW);
}

#[test]
fn a_painted_cell_does_not_shift_the_column_beside_it() {
    let mut status = a_node();
    status.links = vec![
        a_link("tor", Some("first-interface")),
        a_link("covert_icmp", Some("second-interface")),
    ];

    let painted = owo_colors::with_override(true, || {
        all_of_it(&status, Some(Duration::from_secs(1)), &PLAINLY)
    });

    let plain = without_colour(&painted);
    let starts: Vec<usize> = ["first-interface", "second-interface"]
        .iter()
        .filter_map(|name| plain.lines().find_map(|line| line.find(name)))
        .collect();
    assert_eq!(starts.len(), 2, "{painted}");
    assert_eq!(starts[0], starts[1], "{painted}");
}

#[test]
fn the_short_status_counts_the_carriers_instead_of_listing_them() {
    let mut status = a_node();
    status.interfaces = vec![
        a_carrier("one", true),
        a_carrier("two", true),
        a_carrier("three", false),
    ];

    let short = without_colour(&all_of_it(&status, Some(Duration::from_secs(1)), &IN_SHORT));

    assert!(short.contains("direct 2/3"), "{short}");
    assert!(!short.contains("three"), "{short}");
}

#[test]
fn counts_are_scaled_where_a_reader_would_scale_them() {
    assert_eq!(bytes(512), "512 B");
    assert_eq!(bytes(2048), "2.0 kB");
    assert_eq!(bytes(5 * 1024 * 1024), "5.0 MB");
}

#[test]
fn a_transport_that_has_carried_nothing_still_reports_its_paths() {
    let mut status = a_node();
    status.transport = Some(Transport {
        packets_sent: 0,
        packets_received: 0,
        packets_forwarded: 0,
        packets_dropped: 0,
        announces_processed: 0,
    });

    let shown = all_of_it(&status, Some(Duration::from_secs(1)), &PLAINLY);

    assert!(shown.contains("paths 7"), "{shown}");
}
