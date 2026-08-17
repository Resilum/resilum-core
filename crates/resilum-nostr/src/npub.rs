//! Decoding Nostr npub strings to public keys.

/// The prefix is checked, not just the payload: bech32 carries the human
/// readable part alongside the data, and every other Nostr prefix over 32
/// bytes — an `nsec` above all — decodes to something that is not a public
/// key and must not be read as one.
pub(crate) fn to_pubkey(npub: &str) -> Option<[u8; 32]> {
    let (hrp, decoded) = bech32::decode(npub).ok()?;
    if hrp.as_str() != "npub" {
        return None;
    }
    decoded.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_NPUB: &str = "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu";
    const VALID_PUBKEY: [u8; 32] = [
        0x17, 0x16, 0x2c, 0x92, 0x1d, 0xc4, 0xd2, 0x51, 0x8f, 0x9a, 0x10, 0x1d, 0xb3, 0x36, 0x95,
        0xdf, 0x1a, 0xfb, 0x56, 0xab, 0x82, 0xf5, 0xff, 0x3e, 0x5d, 0xa6, 0xee, 0xc3, 0xca, 0x5c,
        0xd9, 0x17,
    ];

    fn encoded_under(prefix: &str, payload: &[u8]) -> String {
        let hrp = bech32::Hrp::parse(prefix).expect("a valid hrp");
        bech32::encode::<bech32::Bech32>(hrp, payload).expect("encodes")
    }

    #[test]
    fn a_valid_npub_decodes_to_the_correct_pubkey() {
        assert_eq!(to_pubkey(VALID_NPUB), Some(VALID_PUBKEY));
    }

    #[test]
    fn a_string_that_is_not_bech32_at_all_returns_none() {
        assert_eq!(to_pubkey("not-a-real-npub"), None);
    }

    #[test]
    fn thirty_two_bytes_under_another_prefix_returns_none() {
        let nsec = encoded_under("nsec", &VALID_PUBKEY);

        assert_eq!(to_pubkey(&nsec), None);
    }

    #[test]
    fn an_npub_with_wrong_payload_length_returns_none() {
        assert_eq!(to_pubkey(&encoded_under("npub", &[1u8; 31])), None);
        assert_eq!(to_pubkey(&encoded_under("npub", &[1u8; 33])), None);
    }
}
