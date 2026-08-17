use std::time::Duration;

use serde_json::Value;

use super::{Source, may_publish};
use crate::bridge::from_mesh::accept_publish;
use crate::config::NostrConfig;
use crate::event::Event;
use crate::registry::Registry;
use crate::subscription::Subscription;

/// The key behind `SUBSCRIBER_NPUB`, as a subscriber signs its subscription
/// with.
const SUBSCRIBER_PUBKEY: [u8; 32] = [
    0x17, 0x16, 0x2c, 0x92, 0x1d, 0xc4, 0xd2, 0x51, 0x8f, 0x9a, 0x10, 0x1d, 0xb3, 0x36, 0x95, 0xdf,
    0x1a, 0xfb, 0x56, 0xab, 0x82, 0xf5, 0xff, 0x3e, 0x5d, 0xa6, 0xee, 0xc3, 0xca, 0x5c, 0xd9, 0x17,
];
const SUBSCRIBER_NPUB: &str = "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu";
const OWNER: [u8; 16] = [0x33; 16];
const STRANGER: [u8; 16] = [0x44; 16];
const NOW: i64 = 1_786_733_083;

fn registered(pubkey: [u8; 32], lxmf: [u8; 16]) -> Registry {
    let registry = Registry::ephemeral(Duration::from_secs(7 * 24 * 3600));
    registry
        .accept(
            Subscription {
                pubkey,
                lxmf,
                created_at: NOW,
            },
            NOW,
        )
        .expect("accepted");
    registry
}

fn personal() -> NostrConfig {
    NostrConfig {
        allow_npubs: vec![SUBSCRIBER_NPUB.to_owned()],
        ..NostrConfig::default()
    }
}

fn wrapped() -> (Event, Value) {
    let wrap = crate::signed::gift_wrap([7u8; 32], NOW, "hello");
    let json = serde_json::to_string(&wrap).expect("re-encodes");
    (
        wrap,
        Value::String(data_encoding::BASE64.encode(json.as_bytes())),
    )
}

/// The decision `offer` makes, in the order it makes it. Composed here so a
/// gate put back on the event's own key fails these tests too.
fn offer(
    cfg: &NostrConfig,
    registry: &Registry,
    source: Source,
    data: &Value,
) -> Result<(), String> {
    may_publish(cfg, registry, source)?;
    accept_publish(data, cfg).map(|_| ())
}

/// Pins the model, not the mechanism: gating on the event's own pubkey
/// refuses every gift wrap a personal bridge's owner sends.
#[test]
fn a_personal_bridge_publishes_its_owners_gift_wrap() {
    let (wrap, data) = wrapped();
    let registry = registered(SUBSCRIBER_PUBKEY, OWNER);

    assert_ne!(
        wrap.pubkey_bytes(),
        Some(SUBSCRIBER_PUBKEY),
        "the wrap must be signed by a key no allow list names, or this proves nothing"
    );

    assert_eq!(
        offer(&personal(), &registry, Source::Recalled(OWNER), &data),
        Ok(())
    );
}

/// The reachability control for the test above: same wrap, same registry,
/// same address — only the listed npub differs, so the pair shows it runs.
#[test]
fn a_personal_bridge_refuses_an_address_registered_to_someone_else() {
    let (_, data) = wrapped();
    let registry = registered([9u8; 32], OWNER);

    let refused =
        offer(&personal(), &registry, Source::Recalled(OWNER), &data).expect_err("not the owner");

    assert!(refused.contains("npub"), "{refused}");
}

#[test]
fn a_personal_bridge_refuses_an_address_it_holds_no_subscription_from() {
    let (_, data) = wrapped();
    let registry = registered(SUBSCRIBER_PUBKEY, OWNER);

    let refused = offer(&personal(), &registry, Source::Recalled(STRANGER), &data)
        .expect_err("an unknown address");

    assert!(refused.contains("address"), "{refused}");
}

/// An empty allow list is a public bridge, and this changes nothing for it:
/// a mesh peer that has never subscribed may still publish.
#[test]
fn a_public_bridge_publishes_for_an_address_it_has_never_heard_of() {
    let (_, data) = wrapped();
    let registry = registered(SUBSCRIBER_PUBKEY, OWNER);

    assert_eq!(
        offer(
            &NostrConfig::default(),
            &registry,
            Source::Recalled(STRANGER),
            &data
        ),
        Ok(())
    );
}

#[test]
fn a_device_holding_a_listed_key_alongside_another_is_still_admitted() {
    let registry = registered(SUBSCRIBER_PUBKEY, OWNER);
    registry
        .accept(
            Subscription {
                pubkey: [0u8; 32],
                lxmf: OWNER,
                created_at: NOW,
            },
            NOW,
        )
        .expect("a second key at the same address");

    assert_eq!(registry.pubkeys_at(&OWNER).len(), 2);
    assert_eq!(
        may_publish(&personal(), &registry, Source::Recalled(OWNER)),
        Ok(())
    );
}
