use super::*;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_window_nobody_has_measured_has_nothing_to_say() {
    assert_eq!(Window::default().least(), None);
}

#[test]
fn a_queue_on_a_busy_link_does_not_become_its_distance() {
    let mut window = Window::default();

    window.measured(ms(8));
    window.measured(ms(242));
    window.measured(ms(98));

    assert_eq!(window.least(), Some(ms(8)));
}

#[test]
fn a_link_that_has_slowed_is_believed_once_the_fast_samples_have_aged_out() {
    let mut window = Window::default();
    for _ in 0..SAMPLES {
        window.measured(ms(20));
    }

    for _ in 0..SAMPLES {
        window.measured(ms(300));
    }

    assert_eq!(window.least(), Some(ms(300)));
}
