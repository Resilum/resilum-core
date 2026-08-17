use super::*;
use crate::bridge::from_mesh::{accept_publish, ack_json};
use crate::bridge::publish::Source;
use crate::bridge::publish::admit::may_publish;
use crate::config::NostrConfig;
use crate::registry::Registry;
use crate::subscription::Subscription;

/// Signed, but kind 30078 — outside the default `publish_kinds`, so
/// `accept_publish` refuses it and its sender is owed an answer.
const SIGNED_KIND_30078: &str = r#"{"id":"e0a482a9abf89cbdb8753acd82ae877867c5138caa6ae113deab9005a47e9598","pubkey":"17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917","created_at":1786733083,"kind":30078,"tags":[["npub","npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu"],["lxmf","00112233445566778899aabbccddeeff"]],"content":"","sig":"f574e6de365df7970aa5f56e2388772bd51dc3404b62a992e0f13a1da461260e02d9f09afe4acd6a374838e19a195094255ddad941fb37b063924aa6dece6a44"}"#;

const REGISTERED_NPUB: &str = "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu";
const REGISTERED_PUBKEY: [u8; 32] = [
    0x17, 0x16, 0x2c, 0x92, 0x1d, 0xc4, 0xd2, 0x51, 0x8f, 0x9a, 0x10, 0x1d, 0xb3, 0x36, 0x95, 0xdf,
    0x1a, 0xfb, 0x56, 0xab, 0x82, 0xf5, 0xff, 0x3e, 0x5d, 0xa6, 0xee, 0xc3, 0xca, 0x5c, 0xd9, 0x17,
];
const REGISTERED_ADDRESS: [u8; 16] = [0x33; 16];
const NOW: i64 = 1_786_733_083;

fn wrapped() -> (crate::event::Event, Value) {
    let wrap = crate::signed::gift_wrap([7u8; 32], NOW, "hello");
    let json = serde_json::to_string(&wrap).expect("re-encodes");
    (
        wrap,
        Value::String(data_encoding::BASE64.encode(json.as_bytes())),
    )
}

#[test]
fn a_refused_event_is_still_answered_by_the_id_its_sender_sent() {
    let data = Value::String(data_encoding::BASE64.encode(SIGNED_KIND_30078.as_bytes()));
    let reason = accept_publish(&data, &NostrConfig::default()).expect_err("refused");

    let (event_id, verdicts) = ack_for(&data, &reason).expect("an answer for the sender");

    assert_eq!(
        event_id,
        "e0a482a9abf89cbdb8753acd82ae877867c5138caa6ae113deab9005a47e9598"
    );
    let ack = ack_json(&event_id, &verdicts);
    assert_eq!(ack["accepted"], 0);
    assert!(
        ack["reason"].as_str().is_some_and(|r| r.contains("kind")),
        "the sender is told why: {ack}"
    );
}

#[test]
fn a_sender_this_bridge_does_not_carry_is_told_so_by_ack() {
    let (wrap, data) = wrapped();
    let cfg = NostrConfig {
        allow_npubs: vec![REGISTERED_NPUB.to_owned()],
        ..NostrConfig::default()
    };
    let registry = Registry::ephemeral(std::time::Duration::from_secs(600));

    let reason = may_publish(&cfg, &registry, Source::Recalled([0x44; 16]))
        .expect_err("an address with no subscription");
    let (event_id, verdicts) = ack_for(&data, &reason).expect("an answer for the sender");

    assert_eq!(event_id, wrap.id);
    let ack = ack_json(&event_id, &verdicts);
    assert_eq!(ack["accepted"], 0);
    assert_eq!(ack["reason"], reason);
}

/// A personal bridge with one registered, allowed subscriber at
/// `REGISTERED_ADDRESS` — the case the verification gate exists for.
fn personal_with_a_registered_subscriber() -> (NostrConfig, Registry) {
    let cfg = NostrConfig {
        allow_npubs: vec![REGISTERED_NPUB.to_owned()],
        ..NostrConfig::default()
    };
    let registry = Registry::ephemeral(std::time::Duration::from_secs(600));
    registry
        .accept(
            Subscription {
                pubkey: REGISTERED_PUBKEY,
                lxmf: REGISTERED_ADDRESS,
                created_at: NOW,
            },
            NOW,
        )
        .expect("accepted");
    (cfg, registry)
}

/// Being registered and allowed is not enough on its own if the message
/// claiming that address is one this node cannot place — and the sender is
/// told why, not left hanging.
#[test]
fn a_personal_bridge_refuses_an_unverified_publish_and_answers_why() {
    let (wrap, data) = wrapped();
    let (cfg, registry) = personal_with_a_registered_subscriber();

    let reason = may_publish(&cfg, &registry, Source::Claimed(REGISTERED_ADDRESS))
        .expect_err("unrecalled, so the address is only claimed");

    let (event_id, verdicts) = ack_for(&data, &reason).expect("an answer for the sender");
    assert_eq!(event_id, wrap.id);
    let ack = ack_json(&event_id, &verdicts);
    assert_eq!(ack["accepted"], 0);
    assert_eq!(ack["reason"], reason);
}

/// The reachability control for the test above: only the identity differs.
#[test]
fn a_personal_bridge_admits_the_same_publish_once_verified() {
    let (cfg, registry) = personal_with_a_registered_subscriber();

    assert_eq!(
        may_publish(&cfg, &registry, Source::Recalled(REGISTERED_ADDRESS)),
        Ok(())
    );
}

/// Refusing an unrecalled sender here would turn away a first-time publisher
/// this node has not heard an announce from yet.
#[test]
fn a_public_bridge_admits_an_unverified_publish() {
    let registry = Registry::ephemeral(std::time::Duration::from_secs(600));

    assert_eq!(
        may_publish(
            &NostrConfig::default(),
            &registry,
            Source::Claimed(REGISTERED_ADDRESS)
        ),
        Ok(())
    );
}

#[test]
fn a_request_carrying_no_event_at_all_has_nothing_to_answer() {
    let data = Value::String(data_encoding::BASE64.encode(b"not an event"));

    assert!(ack_for(&data, "unreadable").is_none());
}
