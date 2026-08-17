//! Routing by the event's own `p` tags, which is all a batched subscription
//! leaves to route on.

use super::super::admit_all;
use super::{Held, NOW, SUBSCRIBER};
use crate::signed::{gift_wrap, gift_wrap_to};

/// Another subscriber sharing the batch, so an event arriving on it has more
/// than one destination it could wrongly be given to.
const ALSO_IN_THE_BATCH: [u8; 32] = [8u8; 32];

#[test]
fn an_event_on_a_batched_subscription_goes_to_the_subscriber_it_names() {
    let held = Held::holding(&[ALSO_IN_THE_BATCH]);
    let event = gift_wrap(SUBSCRIBER, NOW, "for one of the batch");

    let owed = admit_all(&held.stores(), &event, NOW);

    assert_eq!(owed.len(), 1, "one p tag, one destination");
    assert_eq!(owed[0].subscriber, SUBSCRIBER);
}

/// The batch id says nothing about who an event is for, so an event naming
/// nobody registered has no destination to fall back to. Delivering it to
/// the batch would hand every subscriber a relay's choice of traffic.
#[test]
fn an_event_naming_nobody_this_bridge_holds_is_refused() {
    let held = Held::holding(&[ALSO_IN_THE_BATCH]);
    let stranger = gift_wrap([9u8; 32], NOW, "for a mailbox nobody registered");

    assert!(
        stranger.verify().is_ok(),
        "the p tag is the only thing wrong"
    );
    assert!(admit_all(&held.stores(), &stranger, NOW).is_empty());

    let ours = gift_wrap(SUBSCRIBER, NOW, "for a mailbox nobody registered");
    assert_eq!(admit_all(&held.stores(), &ours, NOW).len(), 1);
}

/// Each subscriber has its own dedupe and its own queue ceiling, so each is
/// owed its own copy.
#[test]
fn an_event_naming_two_subscribers_is_owed_to_both() {
    let held = Held::holding(&[ALSO_IN_THE_BATCH]);
    let event = gift_wrap_to(&[SUBSCRIBER, ALSO_IN_THE_BATCH], NOW, "to both");

    let owed = admit_all(&held.stores(), &event, NOW);

    let mut reached: Vec<[u8; 32]> = owed.iter().map(|entry| entry.subscriber).collect();
    reached.sort_unstable();
    assert_eq!(reached, vec![SUBSCRIBER, ALSO_IN_THE_BATCH]);
}

/// A subscriber the bridge does not hold must not stop the one it does from
/// being served off the same event.
#[test]
fn a_stranger_alongside_a_subscriber_does_not_cost_the_subscriber_its_copy() {
    let held = Held::holding(&[ALSO_IN_THE_BATCH]);
    let event = gift_wrap_to(&[[9u8; 32], SUBSCRIBER], NOW, "one of each");

    let owed = admit_all(&held.stores(), &event, NOW);

    assert_eq!(owed.len(), 1);
    assert_eq!(owed[0].subscriber, SUBSCRIBER);
}
