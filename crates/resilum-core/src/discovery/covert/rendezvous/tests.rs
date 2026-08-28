use proptest::prelude::*;

use super::{as_one_msgpack_value, from_one_msgpack_value};

#[test]
fn an_endpoint_reply_survives_the_trip_whole() {
    let payload = b"icmp:198.18.0.1,2001:db8::1";

    let wrapped = as_one_msgpack_value(payload).expect("wrap");

    assert_eq!(
        from_one_msgpack_value(&wrapped).as_deref(),
        Some(&payload[..])
    );
}

#[test]
fn a_reply_sent_as_raw_bytes_is_refused_rather_than_half_read() {
    assert_eq!(from_one_msgpack_value(b"icmp:198.18.0.1"), None);
}

proptest! {
    #[test]
    fn every_payload_round_trips(payload in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let wrapped = as_one_msgpack_value(&payload).expect("wrap");
        let unwrapped = from_one_msgpack_value(&wrapped);
        prop_assert_eq!(unwrapped.as_deref(), Some(&payload[..]));
    }

    #[test]
    fn unwrapping_never_panics_on_garbage(raw in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _ = from_one_msgpack_value(&raw);
    }
}
