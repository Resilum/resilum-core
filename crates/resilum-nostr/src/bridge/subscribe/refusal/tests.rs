use super::*;

#[test]
fn every_refusal_names_a_cause_and_an_action_the_device_can_branch_on() {
    let seen: Vec<String> = [
        Refusal::Malformed,
        Refusal::ClockSkew,
        Refusal::NotCarried,
        Refusal::Replayed,
        Refusal::Stale,
        Refusal::Full,
    ]
    .into_iter()
    .map(Refusal::json)
    .collect();

    assert_eq!(
        seen,
        vec![
            r#"{"reason":"malformed","retry":"never"}"#,
            r#"{"reason":"clock_skew","retry":"fix"}"#,
            r#"{"reason":"not_carried","retry":"elsewhere"}"#,
            r#"{"reason":"replayed","retry":"later"}"#,
            r#"{"reason":"stale","retry":"fix"}"#,
            r#"{"reason":"full","retry":"elsewhere"}"#,
        ]
    );
}

#[test]
fn a_replayed_request_waits_because_no_edit_to_it_can_beat_the_record() {
    assert_eq!(Refusal::Replayed.retry(), Retry::Later);
}

#[test]
fn a_cause_that_is_not_the_devices_fault_never_says_never() {
    for refusal in [
        Refusal::ClockSkew,
        Refusal::NotCarried,
        Refusal::Replayed,
        Refusal::Stale,
        Refusal::Full,
    ] {
        assert_ne!(refusal.retry(), Retry::Never, "{refusal:?}");
    }
}
