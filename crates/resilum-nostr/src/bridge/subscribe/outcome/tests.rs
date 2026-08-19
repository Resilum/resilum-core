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
    .map(|refusal| Outcome::Refused(refusal).json())
    .collect();

    assert_eq!(
        seen,
        vec![
            r#"{"reason":"malformed","result":"refused","retry":"never"}"#,
            r#"{"reason":"clock_skew","result":"refused","retry":"fix"}"#,
            r#"{"reason":"not_carried","result":"refused","retry":"elsewhere"}"#,
            r#"{"reason":"replayed","result":"refused","retry":"later"}"#,
            r#"{"reason":"stale","result":"refused","retry":"fix"}"#,
            r#"{"reason":"full","result":"refused","retry":"elsewhere"}"#,
        ]
    );
}

#[test]
fn an_accepted_subscription_says_so_and_names_nothing_to_retry() {
    assert_eq!(Outcome::Accepted.json(), r#"{"result":"accepted"}"#);
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
