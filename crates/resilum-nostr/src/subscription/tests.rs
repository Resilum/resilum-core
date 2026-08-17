use data_encoding::HEXLOWER;

use super::*;

const NOW: i64 = 1_786_733_083;
const FROM_AN_INDEPENDENT_CLIENT: &str = r#"{"id":"e0a482a9abf89cbdb8753acd82ae877867c5138caa6ae113deab9005a47e9598","pubkey":"17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917","created_at":1786733083,"kind":30078,"tags":[["npub","npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu"],["lxmf","00112233445566778899aabbccddeeff"]],"content":"","sig":"f574e6de365df7970aa5f56e2388772bd51dc3404b62a992e0f13a1da461260e02d9f09afe4acd6a374838e19a195094255ddad941fb37b063924aa6dece6a44"}"#;

fn subscriber_event() -> Event {
    crate::event::parse(FROM_AN_INDEPENDENT_CLIENT).expect("parses")
}

#[test]
fn a_subscription_from_another_implementation_is_accepted() {
    let sub = from_event(&subscriber_event(), NOW).expect("accepted");

    assert_eq!(
        HEXLOWER.encode(&sub.pubkey),
        "17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917"
    );
    assert_eq!(sub.lxmf[0], 0x00);
    assert_eq!(sub.lxmf[15], 0xff);
    assert_eq!(sub.created_at, NOW);
}

/// The npub tag is what the relay would otherwise trust blindly; if it
/// disagrees with the signing key, someone is redirecting another
/// person's mail.
#[test]
fn an_npub_that_disagrees_with_the_signing_key_is_refused() {
    let mut event = subscriber_event();
    let other_key = [1u8; 32];
    let other_npub =
        bech32::encode::<bech32::Bech32>(bech32::Hrp::parse("npub").unwrap(), &other_key).unwrap();
    event.tags[0][1] = other_npub;

    assert!(from_event(&event, NOW).is_err());
}

/// Both sides of the bound, and spelled out rather than written in terms of
/// `FUTURE_SKEW_SECS`: a test that names the constant moves with it, and
/// would go on passing at any tolerance including none at all.
#[test]
fn a_subscriber_clock_is_tolerated_to_fifteen_minutes_and_no_further() {
    let event = subscriber_event();

    assert!(from_event(&event, NOW - 900).is_ok(), "900 seconds ahead");
    assert!(from_event(&event, NOW - 901).is_err(), "901 seconds ahead");
}

#[test]
fn an_explicit_filter_is_refused() {
    let mut event = subscriber_event();
    event.tags.push(vec!["filter".into(), "{}".into()]);

    assert!(from_event(&event, NOW).is_err());
}

#[test]
fn an_lxmf_tag_in_uppercase_still_names_its_mailbox() {
    let shouted = subscribing_from("00112233445566778899AABBCCDDEEFF");
    assert!(shouted.verify().is_ok(), "the casing is the only oddity");

    let sub = from_event(&shouted, NOW).expect("accepted");

    let spoken = subscribing_from("00112233445566778899aabbccddeeff");
    let plain = from_event(&spoken, NOW).expect("accepted");
    assert_eq!(sub.lxmf, plain.lxmf, "the same mailbox either way");
    assert_eq!(sub.lxmf[10], 0xaa);
}

fn subscribing_from(lxmf: &str) -> Event {
    let hrp = bech32::Hrp::parse("npub").expect("a valid hrp");
    let npub = bech32::encode::<bech32::Bech32>(hrp, &crate::signed::pubkey())
        .expect("the fixture key encodes");
    crate::signed::sign_tags(
        &crate::signed::Draft {
            kind: KIND,
            addressed_to: &[],
            created_at: NOW,
            content: "",
        },
        vec![vec!["npub".into(), npub], vec!["lxmf".into(), lxmf.into()]],
    )
}
