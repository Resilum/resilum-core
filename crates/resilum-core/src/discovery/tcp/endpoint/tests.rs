use super::*;

fn onion() -> EndpointFormat {
    EndpointFormat::Base32 {
        suffix: ".onion".into(),
        label_len: 35,
    }
}

fn i2p() -> EndpointFormat {
    EndpointFormat::Base32 {
        suffix: ".b32.i2p".into(),
        label_len: 32,
    }
}

fn label_of(bytes: &[u8]) -> String {
    data_encoding::BASE32_NOPAD
        .encode(bytes)
        .to_ascii_lowercase()
}

/// A real v3 label: 35 bytes of key, checksum and version, so 56 characters.
fn onion_host() -> String {
    format!("{}.onion", label_of(&[0x5a; 35]))
}

#[test]
fn an_onion_address_comes_back_character_for_character() {
    let host = onion_host();
    let wire = encode_endpoint(&host, 4242, &onion()).expect("encodes");

    assert_eq!(wire.len(), 37, "35 bytes of label plus the port");
    assert_eq!(parse_endpoint(&wire, &onion()), Some((host, 4242)));
}

#[test]
fn an_i2p_address_comes_back_character_for_character() {
    let host = format!("{}.b32.i2p", label_of(&[0x3c; 32]));
    let wire = encode_endpoint(&host, 8000, &i2p()).expect("encodes");

    assert_eq!(wire.len(), 34, "32 bytes of hash plus the port");
    assert_eq!(parse_endpoint(&wire, &i2p()), Some((host, 8000)));
}

#[test]
fn an_ipv6_address_comes_back_in_its_canonical_form() {
    let wire = encode_endpoint(
        "201:86cc:eddf:6f6f:1151:15ce:9864:d45b",
        4242,
        &EndpointFormat::BracketedIpv6,
    )
    .expect("encodes");

    assert_eq!(wire.len(), 18, "16 bytes of address plus the port");
    assert_eq!(
        parse_endpoint(&wire, &EndpointFormat::BracketedIpv6),
        Some(("201:86cc:eddf:6f6f:1151:15ce:9864:d45b".to_owned(), 4242))
    );
}

#[test]
fn the_binary_form_is_shorter_than_the_text_it_replaces() {
    let host = onion_host();
    let wire = encode_endpoint(&host, 4242, &onion()).expect("encodes");

    assert!(wire.len() < format!("{host}:4242").len());
}

#[test]
fn a_host_without_the_expected_suffix_is_refused() {
    assert!(encode_endpoint("peer.b32.i2p", 4242, &onion()).is_none());
    assert!(encode_endpoint("not-an-address", 4242, &EndpointFormat::BracketedIpv6).is_none());
}

#[test]
fn a_payload_of_the_wrong_shape_is_refused() {
    assert!(parse_endpoint(b"", &onion()).is_none());
    assert!(parse_endpoint(&[0x01], &onion()).is_none());
    // 35 bytes of label plus the port is what an onion address is.
    assert!(parse_endpoint(&[0x01; 37], &onion()).is_some());
    // An i2p label is shorter, so it is not an onion one.
    assert!(parse_endpoint(&[0x01; 34], &onion()).is_none());
    // Port zero cannot be dialled.
    assert!(parse_endpoint(&[0u8; 37], &onion()).is_none());
    // An IPv6 address is exactly 16 bytes.
    assert!(parse_endpoint(&[0x01; 17], &EndpointFormat::BracketedIpv6).is_none());
}
