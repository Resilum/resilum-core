use super::*;

/// The room a ratcheted announce leaves for app data, from the stack that
/// enforces it rather than copied here.
fn budget() -> usize {
    leviculum_core::announce_app_data_budget(true)
}

fn endpoints(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    pairs
        .iter()
        .map(|(s, e)| ((*s).to_owned(), e.to_vec()))
        .collect()
}

#[test]
fn every_ready_transport_rides_in_one_payload() {
    let eps = endpoints(&[("tor", b"onion-key"), ("yggdrasil", b"ygg-addr")]);
    let (packed, left_out) = pack(&eps, "*", budget());
    let parsed = parse(&packed).unwrap();

    assert!(left_out.is_empty());
    assert_eq!(parsed.endpoints, eps);
    assert_eq!(parsed.exit_country, "*");
}

#[test]
fn a_payload_without_endpoints_still_carries_the_country() {
    let (packed, _) = pack(&BTreeMap::new(), "NL", budget());
    let parsed = parse(&packed).unwrap();

    assert!(parsed.endpoints.is_empty());
    assert_eq!(parsed.exit_country, "NL");
}

#[test]
fn missing_country_defaults_to_wildcard() {
    let (packed, _) = pack(&BTreeMap::new(), "", budget());
    assert_eq!(parse(&packed).unwrap().exit_country, "*");
}

#[test]
fn drops_incompatible_malformed_and_empty() {
    let newer = rmp_serde::to_vec(&Wire(WIRE_VERSION + 1, BTreeMap::new(), None)).unwrap();
    assert!(parse(&newer).is_none());
    assert!(parse(b"not msgpack").is_none());
    assert!(parse(b"").is_none());
}

/// An announce past the budget is refused whole, so the transports that did
/// fit have to survive the one that did not.
#[test]
fn a_transport_that_does_not_fit_is_left_out_rather_than_losing_the_rest() {
    let eps = endpoints(&[
        ("tor", &[0xaa; 34]),
        ("i2p", &[0xbb; 34]),
        ("yggdrasil", &[0xcc; 18]),
        ("iroh", &[0xdd; 200]),
    ]);
    let (packed, left_out) = pack(&eps, "*", budget());

    assert!(packed.len() <= budget(), "packed {} bytes", packed.len());
    assert_eq!(left_out, vec!["iroh".to_owned()]);
    let kept = parse(&packed).unwrap().endpoints;
    assert_eq!(kept.len(), 3);
    assert!(kept.contains_key("tor"));
}

#[test]
fn a_service_with_no_wire_id_is_reported_rather_than_sent_unnamed() {
    let eps = endpoints(&[("tor", b"key"), ("nostr", b"whatever")]);
    let (packed, left_out) = pack(&eps, "*", budget());

    assert_eq!(left_out, vec!["nostr".to_owned()]);
    assert_eq!(parse(&packed).unwrap().endpoints.len(), 1);
}

/// The transports this node can offer today, at their binary sizes, with room
/// left for the ones the roadmap adds.
#[test]
fn every_service_fits_with_room_to_spare() {
    let mut eps = BTreeMap::new();
    for (service, len) in [
        ("tor", 37),
        ("i2p", 34),
        ("yggdrasil", 18),
        ("iroh", 57),
        ("covert_icmp", 9),
    ] {
        eps.insert(service.to_owned(), vec![0x11; len]);
    }
    let (packed, left_out) = pack(&eps, "*", budget());

    assert!(left_out.is_empty(), "left out {left_out:?}");
    // Room for several more carriers, which are a dozen bytes each.
    let spare = budget() - packed.len();
    assert!(spare >= 100, "packed {} bytes, {spare} spare", packed.len());
}

/// A node running everything it can today: onion, i2p, yggdrasil and a QUIC
/// endpoint with a relay URL. All four have to reach a peer in one announce.
#[test]
fn a_node_with_every_transport_fits_one_announce() {
    let iroh_with_relay = {
        let mut v = vec![0x77; 32];
        v.extend_from_slice(b"https://euw1-1.relay.iroh.network./");
        v
    };
    let eps = endpoints(&[
        ("tor", &[0x5a; 37]),
        ("i2p", &[0x3c; 34]),
        ("yggdrasil", &[0x20; 18]),
        ("iroh", &iroh_with_relay),
    ]);
    let (packed, left_out) = pack(&eps, "*", budget());

    assert!(left_out.is_empty(), "left out {left_out:?}");
    assert!(packed.len() <= budget(), "packed {} bytes", packed.len());
}
