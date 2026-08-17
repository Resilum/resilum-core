use super::*;

/// A real signature over kind 30078, which is outside the default
/// `publish_kinds` ([1059, 4]). A validly-signed fixture proves the refusal
/// below is the kind check and not verification failing first.
const SIGNED_KIND_30078: &str = r#"{"id":"e0a482a9abf89cbdb8753acd82ae877867c5138caa6ae113deab9005a47e9598","pubkey":"17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917","created_at":1786733083,"kind":30078,"tags":[["npub","npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu"],["lxmf","00112233445566778899aabbccddeeff"]],"content":"","sig":"f574e6de365df7970aa5f56e2388772bd51dc3404b62a992e0f13a1da461260e02d9f09afe4acd6a374838e19a195094255ddad941fb37b063924aa6dece6a44"}"#;

#[test]
fn an_event_of_a_kind_this_bridge_does_not_carry_is_refused() {
    let cfg = NostrConfig::default();
    let data = Value::String(data_encoding::BASE64.encode(SIGNED_KIND_30078.as_bytes()));

    let refused = accept_publish(&data, &cfg).expect_err("kind 30078 is not carried");
    assert!(refused.contains("kind"));
}

/// The default `publish_kinds` never includes 30078; a config that adds it
/// must let this fixture through.
#[test]
fn a_kind_the_config_allows_beyond_the_default_is_carried() {
    let cfg = NostrConfig {
        publish_kinds: vec![30078],
        ..NostrConfig::default()
    };
    let data = Value::String(data_encoding::BASE64.encode(SIGNED_KIND_30078.as_bytes()));

    assert!(accept_publish(&data, &cfg).is_ok());
}

/// The default `publish_kinds` always includes 1059; a config that narrows it
/// must refuse a gift wrap despite the default allowing it.
#[test]
fn a_kind_the_default_allows_but_the_config_excludes_is_refused() {
    let wrap = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "hello");
    let json = serde_json::to_string(&wrap).expect("re-encodes");
    let data = Value::String(data_encoding::BASE64.encode(json.as_bytes()));
    let cfg = NostrConfig {
        publish_kinds: vec![4],
        ..NostrConfig::default()
    };

    let refused = accept_publish(&data, &cfg).expect_err("1059 is excluded by this config");
    assert!(refused.contains("kind"));
}

#[test]
fn a_gift_wrap_is_carried_whoever_its_one_time_key_belongs_to() {
    let wrap = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "hello");
    let json = serde_json::to_string(&wrap).expect("re-encodes");
    let data = Value::String(data_encoding::BASE64.encode(json.as_bytes()));

    let personal = NostrConfig {
        allow_npubs: vec![
            "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu".to_owned(),
        ],
        ..NostrConfig::default()
    };

    assert!(accept_publish(&data, &personal).is_ok());
}
