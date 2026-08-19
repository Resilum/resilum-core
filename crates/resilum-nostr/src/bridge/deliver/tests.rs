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
