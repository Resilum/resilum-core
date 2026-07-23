//! Rendezvous endpoint wire form: `<carrier>:<addr1[,addr2,...]>`.

pub fn pack(carrier: &str, addrs: &[String]) -> Vec<u8> {
    format!("{carrier}:{}", addrs.join(",")).into_bytes()
}

pub fn parse(raw: &[u8]) -> Option<(String, Vec<String>)> {
    let s = std::str::from_utf8(raw).ok()?;
    let (carrier, rest) = s.split_once(':')?;
    if carrier.is_empty() || rest.is_empty() {
        return None;
    }
    let addrs: Vec<String> = rest.split(',').map(str::to_owned).collect();
    if addrs.iter().any(String::is_empty) {
        return None;
    }
    Some((carrier.to_owned(), addrs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn round_trips_a_single_v4_addr() {
        let raw = pack("icmp", &["203.0.113.9".into()]);
        let (c, a) = parse(&raw).unwrap();
        assert_eq!(c, "icmp");
        assert_eq!(a, vec!["203.0.113.9"]);
    }

    #[test]
    fn round_trips_csv_v6_and_v4() {
        let raw = pack("icmp", &["2001:db8::1".into(), "10.0.0.1".into()]);
        let (c, a) = parse(&raw).unwrap();
        assert_eq!(c, "icmp");
        assert_eq!(a, vec!["2001:db8::1", "10.0.0.1"]);
    }

    #[test]
    fn rejects_empty_carrier_or_addrs() {
        assert!(parse(b":1.2.3.4").is_none());
        assert!(parse(b"icmp:").is_none());
        assert!(parse(b"icmp:1.2.3.4,").is_none());
    }

    proptest! {
        #[test]
        fn roundtrip_never_loses_data(
            carrier in "[a-z]{1,10}",
            addrs in proptest::collection::vec("[a-zA-Z0-9.:]{1,40}", 1..5),
        ) {
            let raw = pack(&carrier, &addrs);
            let (c, a) = parse(&raw).expect("valid input must round-trip");
            prop_assert_eq!(c, carrier);
            prop_assert_eq!(a, addrs);
        }

        #[test]
        fn parse_never_panics_on_garbage(raw in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = parse(&raw);
        }
    }
}
