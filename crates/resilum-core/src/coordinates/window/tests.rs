use super::*;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_window_nobody_has_measured_has_nothing_to_say() {
    assert_eq!(Window::default().dependably_fast(), None);
}

#[test]
fn a_queue_on_a_busy_link_does_not_become_its_distance() {
    let mut window = Window::default();

    window.measured(ms(8), 100.0);
    window.measured(ms(242), 120.0);
    window.measured(ms(98), 140.0);

    assert_eq!(window.dependably_fast(), Some(ms(8)));
}

#[test]
fn a_way_through_that_opened_once_in_eight_tries_is_not_this_peers_distance() {
    let mut window = Window::default();
    window.measured(ms(4), 100.0);
    for round in 1..8 {
        window.measured(ms(900), 100.0 + f64::from(round));
    }

    assert_eq!(window.dependably_fast(), Some(ms(900)));
}

#[test]
fn a_way_through_that_opens_every_third_try_is_this_peers_distance() {
    let mut window = Window::default();
    let mut at = 100.0;
    for rtt in [64, 475, 64, 907, 65, 2097, 2678] {
        window.measured(ms(rtt), at);
        at += 1.0;
    }

    assert_eq!(window.dependably_fast(), Some(ms(64)));
}

#[test]
fn a_link_that_has_slowed_is_believed_once_the_fast_samples_have_aged_out() {
    let mut window = Window::default();
    window.measured(ms(20), 100.0);

    window.measured(ms(300), 100.0 + SEEN_WITHIN + 1.0);

    assert_eq!(window.dependably_fast(), Some(ms(300)));
}

#[test]
fn a_sample_from_before_the_window_is_forgotten_however_slowly_this_node_asks() {
    let mut window = Window::default();
    window.measured(ms(20), 100.0);

    window.measured(ms(300), 100.0 + SEEN_WITHIN / 2.0);
    assert_eq!(
        window.dependably_fast(),
        Some(ms(20)),
        "still inside the window"
    );

    window.measured(ms(300), 100.0 + SEEN_WITHIN + 1.0);
    assert_eq!(window.dependably_fast(), Some(ms(300)));
}
