use super::*;

fn sample() -> Datagram {
    Datagram {
        session: 0x11223344,
        seq: 42,
        ack: 7,
        kind: Kind::Data,
        payload: b"hello".to_vec(),
    }
}

#[test]
fn round_trip_authenticated() {
    let key = b"secret-key";
    let raw = pack(&sample(), key, DEFAULT_TAG_LEN);
    let back = unpack(&raw, key, DEFAULT_TAG_LEN).unwrap();
    assert_eq!(back, sample());
}

#[test]
fn wrong_key_rejects() {
    let raw = pack(&sample(), b"k1", DEFAULT_TAG_LEN);
    assert!(unpack(&raw, b"k2", DEFAULT_TAG_LEN).is_none());
}

#[test]
fn short_frames_rejected() {
    assert!(
        unpack(
            &[0u8; HEADER_LEN + DEFAULT_TAG_LEN - 1],
            b"k",
            DEFAULT_TAG_LEN
        )
        .is_none()
    );
}

#[test]
fn peek_gets_kind_and_session() {
    let raw = pack(&sample(), b"k", 0);
    let (session, seq, _ack, kind_byte) = peek_header(&raw).unwrap();
    assert_eq!(session, 0x11223344);
    assert_eq!(seq, 42);
    assert_eq!(Kind::from_u8(kind_byte), Some(Kind::Data));
}
