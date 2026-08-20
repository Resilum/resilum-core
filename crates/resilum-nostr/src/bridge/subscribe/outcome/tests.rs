use super::*;

#[test]
fn every_refusal_names_a_cause_and_an_action_the_device_can_branch_on() {
    let seen: Vec<Value> = [
        Refusal::Malformed,
        Refusal::ClockSkew,
        Refusal::NotCarried,
        Refusal::Replayed,
        Refusal::Stale,
        Refusal::Full,
    ]
    .into_iter()
    .map(|refusal| Outcome::Refused(refusal).body())
    .collect();

    assert_eq!(
        seen,
        vec![
            json!({"result": "refused", "reason": "malformed", "retry": "never"}),
            json!({"result": "refused", "reason": "clock_skew", "retry": "fix"}),
            json!({"result": "refused", "reason": "not_carried", "retry": "elsewhere"}),
            json!({"result": "refused", "reason": "replayed", "retry": "later"}),
            json!({"result": "refused", "reason": "stale", "retry": "fix"}),
            json!({"result": "refused", "reason": "full", "retry": "elsewhere"}),
        ]
    );
}

#[test]
fn an_accepted_subscription_names_the_relays_it_will_be_read_on() {
    let accepted = Outcome::Accepted {
        read_on: vec![
            "wss://one.example".to_owned(),
            "wss://two.example".to_owned(),
        ],
    };

    assert_eq!(
        accepted.body(),
        json!({
            "result": "accepted",
            "relays": ["wss://one.example", "wss://two.example"],
        })
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
