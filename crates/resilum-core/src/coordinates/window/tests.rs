use super::*;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_window_nobody_has_measured_has_no_typical_value() {
    assert_eq!(Window::default().typical(), None);
}

#[test]
fn one_queued_packet_does_not_become_the_typical_value() {
    let mut window = Window::default();
    for _ in 0..5 {
        window.measured(ms(20));
    }

    window.measured(ms(4000));

    assert_eq!(window.typical(), Some(ms(20)));
}

#[test]
fn a_link_that_has_slowed_is_believed_once_the_window_agrees() {
    let mut window = Window::default();
    for _ in 0..SAMPLES {
        window.measured(ms(20));
    }

    for _ in 0..SAMPLES {
        window.measured(ms(300));
    }

    assert_eq!(window.typical(), Some(ms(300)));
}
