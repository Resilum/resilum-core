mod filter;
mod route;

use std::time::Duration;

use super::*;
use crate::queue::Queue;
use crate::signed::gift_wrap;
use crate::subscription::Subscription;

const SUBSCRIBER: [u8; 32] = [7u8; 32];
const SUBSCRIBED_AT: i64 = 1_786_000_000;
const NOW: i64 = 1_786_733_083;

struct Held {
    cfg: NostrConfig,
    registry: Registry,
    queue: Queue,
    recent: Recent,
}

impl Held {
    fn new() -> Self {
        Self::configured(NostrConfig::default())
    }

    fn configured(cfg: NostrConfig) -> Self {
        let held = Self {
            cfg,
            registry: Registry::ephemeral(Duration::from_secs(7 * 24 * 3600)),
            queue: Queue::ephemeral(Duration::from_secs(7 * 24 * 3600), 256),
            recent: Recent::default(),
        };
        held.register(SUBSCRIBER);
        held
    }

    /// A bridge carrying `SUBSCRIBER` and the others too, which is what puts
    /// them on one batched subscription.
    fn holding(others: &[[u8; 32]]) -> Self {
        let held = Self::new();
        for pubkey in others {
            held.register(*pubkey);
        }
        held
    }

    fn register(&self, pubkey: [u8; 32]) {
        self.registry
            .accept(
                Subscription {
                    pubkey,
                    lxmf: [0x33; 16],
                    created_at: SUBSCRIBED_AT,
                },
                SUBSCRIBED_AT,
            )
            .expect("accepted");
    }

    fn stores(&self) -> Stores<'_> {
        Stores {
            cfg: &self.cfg,
            registry: &self.registry,
            queue: &self.queue,
            recent: &self.recent,
        }
    }
}

/// A gift wrap is unauthenticated and addressed only by its `#p` tag, so
/// anyone may publish one dated whenever they like.
#[test]
fn an_event_from_the_year_2100_does_not_carry_the_mark_with_it() {
    let held = Held::new();
    let event = gift_wrap(SUBSCRIBER, 4_102_444_800, "from the future");

    assert!(admit(&held.stores(), SUBSCRIBER, &event, NOW).is_some());

    assert_eq!(held.registry.last_seen(&SUBSCRIBER), Some(NOW));
}

/// Rewinding the mark is re-delivery of everything since, on every reconnect.
#[test]
fn an_older_event_does_not_pull_the_mark_backwards() {
    let held = Held::new();
    let recent = gift_wrap(SUBSCRIBER, NOW, "recent");
    admit(&held.stores(), SUBSCRIBER, &recent, NOW).expect("admitted");
    assert_eq!(held.registry.last_seen(&SUBSCRIBER), Some(NOW));

    let older = gift_wrap(SUBSCRIBER, NOW - 5_000, "older");
    admit(&held.stores(), SUBSCRIBER, &older, NOW).expect("admitted");

    assert_eq!(held.registry.last_seen(&SUBSCRIBER), Some(NOW));
}

#[test]
fn an_event_that_does_not_verify_is_not_owed_to_anyone() {
    let held = Held::new();
    let mut event = gift_wrap(SUBSCRIBER, NOW, "forged");
    event.sig = "0".repeat(128);

    assert!(admit(&held.stores(), SUBSCRIBER, &event, NOW).is_none());
    assert_eq!(held.registry.last_seen(&SUBSCRIBER), Some(SUBSCRIBED_AT));
}

/// `since` is inclusive, so the next reconnect re-serves the event the mark
/// stands on — by which time the queue has long dropped it as delivered.
#[test]
fn an_event_already_delivered_is_not_sent_a_second_time() {
    let held = Held::new();
    let event = gift_wrap(SUBSCRIBER, NOW, "delivered once");
    let entry = admit(&held.stores(), SUBSCRIBER, &event, NOW).expect("admitted");

    held.queue.resolve(&entry.event_id, &entry.subscriber);

    assert!(admit(&held.stores(), SUBSCRIBER, &event, NOW).is_none());
}
