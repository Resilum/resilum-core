use super::*;

const SUBSCRIBER_PUBKEY: [u8; 32] = [
    0x17, 0x16, 0x2c, 0x92, 0x1d, 0xc4, 0xd2, 0x51, 0x8f, 0x9a, 0x10, 0x1d, 0xb3, 0x36, 0x95, 0xdf,
    0x1a, 0xfb, 0x56, 0xab, 0x82, 0xf5, 0xff, 0x3e, 0x5d, 0xa6, 0xee, 0xc3, 0xca, 0x5c, 0xd9, 0x17,
];
const SUBSCRIBER_NPUB: &str = "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu";

#[test]
fn an_empty_allow_list_is_an_open_bridge() {
    let cfg = NostrConfig::default();

    assert!(cfg.may_use(&SUBSCRIBER_PUBKEY));
    assert!(cfg.may_use(&[9u8; 32]));
}

#[test]
fn a_named_subscriber_is_the_only_one_admitted() {
    let cfg = NostrConfig {
        allow_npubs: vec![SUBSCRIBER_NPUB.to_owned()],
        ..NostrConfig::default()
    };

    assert!(cfg.may_use(&SUBSCRIBER_PUBKEY));
    assert!(!cfg.may_use(&[9u8; 32]));
}

#[test]
fn legacy_delivery_widens_only_the_inbound_side() {
    let cfg = NostrConfig::default();
    assert_eq!(cfg.inbound_kinds(), vec![1059, 4]);

    let strict = NostrConfig {
        legacy_dm: false,
        ..NostrConfig::default()
    };
    assert_eq!(strict.inbound_kinds(), vec![1059]);
    assert_eq!(strict.publish_kinds, vec![1059, 4]);
}

#[test]
fn the_retention_of_the_specification_parses() {
    let cfg: NostrConfig =
        serde_yaml_ng::from_str("upstreams: [wss://relay.example]\nretention: 7d\n")
            .expect("parses");

    assert_eq!(cfg.retention, Duration::from_secs(7 * 24 * 3600));
}

#[test]
fn a_misspelled_allow_list_key_is_refused_rather_than_read_as_an_open_bridge() {
    for typo in ["allow_npub", "allowed_npubs"] {
        let yaml = format!("{typo}:\n  - {SUBSCRIBER_NPUB}\n");

        let parsed = serde_yaml_ng::from_str::<NostrConfig>(&yaml);

        assert!(parsed.is_err(), "{typo} was accepted and admits everyone");
    }
}

#[test]
fn an_invalid_npub_in_allow_list_admits_nobody_and_does_not_panic() {
    let cfg = NostrConfig {
        allow_npubs: vec!["garbage".to_owned()],
        ..NostrConfig::default()
    };

    assert!(!cfg.may_use(&SUBSCRIBER_PUBKEY));
    assert!(!cfg.may_use(&[9u8; 32]));
}

/// A field defaulted in one place and not the other would have a bridge
/// started from an empty file behave unlike one started from `Default`.
#[test]
fn defaults_are_consistent_between_impl_and_serde() {
    let from_impl = NostrConfig::default();
    let from_empty_yaml: NostrConfig = serde_yaml_ng::from_str("").expect("parses");

    assert_eq!(from_impl, from_empty_yaml);
}
