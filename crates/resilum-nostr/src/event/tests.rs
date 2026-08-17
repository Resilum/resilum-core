use super::*;

/// Captured from an independent client implementation: regenerating it with
/// our own signer would make this self-consistency, not interoperability.
const FROM_AN_INDEPENDENT_CLIENT: &str = r#"{"id":"e0a482a9abf89cbdb8753acd82ae877867c5138caa6ae113deab9005a47e9598","pubkey":"17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917","created_at":1786733083,"kind":30078,"tags":[["npub","npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu"],["lxmf","00112233445566778899aabbccddeeff"]],"content":"","sig":"f574e6de365df7970aa5f56e2388772bd51dc3404b62a992e0f13a1da461260e02d9f09afe4acd6a374838e19a195094255ddad941fb37b063924aa6dece6a44"}"#;

#[test]
fn an_event_from_another_implementation_verifies_here() {
    let event = parse(FROM_AN_INDEPENDENT_CLIENT).expect("parses");

    assert!(event.verify().is_ok());
    assert_eq!(event.tag("lxmf"), Some("00112233445566778899aabbccddeeff"));
}

#[test]
fn an_edited_event_no_longer_matches_its_id() {
    let mut event = parse(FROM_AN_INDEPENDENT_CLIENT).expect("parses");
    event.tags[1][1] = "ffffffffffffffffffffffffffffffff".into();

    assert_eq!(event.verify(), Err(VerifyError::IdMismatch));
}

/// `publish_frame` ships these exact bytes on to the relays, where NIP-01
/// says the id is lowercase hex.
#[test]
fn an_event_whose_id_is_uppercase_hex_is_refused() {
    let mut event = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "shouted");
    assert!(event.verify().is_ok(), "the casing is the only oddity");

    event.id = event.id.to_uppercase();

    assert_eq!(event.verify(), Err(VerifyError::IdNotHex));
}

#[test]
fn a_signature_lifted_from_another_event_is_refused() {
    let mut event = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "one");
    let donor = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, "another");
    assert!(
        donor.verify().is_ok(),
        "the donor's signature is a real one"
    );

    event.sig = donor.sig;

    assert_eq!(event.verify(), Err(VerifyError::SignatureMismatch));
}
