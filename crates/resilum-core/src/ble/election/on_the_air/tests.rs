use super::{STOPS_COUNTING_AFTER_MS, SomeoneElsesGroup};

#[test]
fn a_node_that_has_heard_nothing_sees_no_group() {
    let heard = SomeoneElsesGroup::none_heard_yet();

    heard.forget_it_if_it_has_gone_quiet(STOPS_COUNTING_AFTER_MS * 4);

    assert!(!heard.is_up());
}

#[test]
fn a_beacon_just_heard_puts_a_group_on_the_air() {
    let heard = SomeoneElsesGroup::none_heard_yet();

    heard.heard_at(1_000);
    heard.forget_it_if_it_has_gone_quiet(1_000 + STOPS_COUNTING_AFTER_MS - 1);

    assert!(heard.is_up());
}

#[test]
fn a_host_that_went_quiet_stops_counting() {
    let heard = SomeoneElsesGroup::none_heard_yet();

    heard.heard_at(1_000);
    heard.forget_it_if_it_has_gone_quiet(1_000 + STOPS_COUNTING_AFTER_MS);

    assert!(!heard.is_up());
}

#[test]
fn hearing_it_again_keeps_the_group_alive() {
    let heard = SomeoneElsesGroup::none_heard_yet();

    heard.heard_at(1_000);
    heard.heard_at(1_000 + STOPS_COUNTING_AFTER_MS - 1);
    heard.forget_it_if_it_has_gone_quiet(1_000 + STOPS_COUNTING_AFTER_MS);

    assert!(heard.is_up());
}

#[test]
fn a_radio_that_stopped_looking_leaves_no_group_behind() {
    let heard = SomeoneElsesGroup::none_heard_yet();

    heard.heard_at(1_000);
    heard.forget_it();

    assert!(!heard.is_up());
}
