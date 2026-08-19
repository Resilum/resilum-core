use super::*;

fn ladder(rungs: usize) -> Vec<Option<(&'static str, Handoff)>> {
    let mut handoff = Handoff::default();
    (0..rungs)
        .map(|_| {
            let (method, next) = on_schedule(handoff)?;
            handoff = next;
            Some((method.token(), handoff))
        })
        .collect()
}

#[test]
fn an_acknowledgement_carries_its_body_as_an_object_and_not_as_json_inside_json() {
    let message = acknowledgement([0xaa; 16], SCHEMA_SUBSCRIBE_ACK, Outcome::Accepted.body());

    let message: Value = serde_json::from_str(&message).expect("the message is json");
    assert_eq!(
        message["fields"]["custom_data"],
        json!({ "result": "accepted" })
    );
}

#[test]
fn two_direct_tries_are_followed_by_the_mailbox_and_then_the_schedule_stops() {
    assert_eq!(
        ladder(4),
        vec![
            Some(("direct", Handoff::Direct { tries: 1 })),
            Some(("direct", Handoff::Direct { tries: 2 })),
            Some(("propagated", Handoff::LeftWithPropagationNode)),
            None,
        ]
    );
}
