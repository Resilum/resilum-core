use super::*;

#[test]
fn checksum_matches_known_ping() {
    // "abcdefgh" payload in an echo-request with id=1234, seq=0.
    let body = build_echo_request(1234, b"abcdefgh", false);
    assert_eq!(body[0], 8); // request
    assert_eq!(u16::from_be_bytes([body[4], body[5]]), 1234);
    // Checksum re-verified over the whole buffer must be zero (per RFC 1071).
    let mut buf = body.clone();
    let mut sum: u32 = 0;
    for c in buf.chunks_exact(2) {
        sum = sum.wrapping_add(u16::from_be_bytes([c[0], c[1]]) as u32);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    assert_eq!(sum as u16, 0xFFFF, "checksum verification failed");
    // Ensure the payload survived.
    let tail_start = 8;
    assert_eq!(&buf.split_off(tail_start), b"abcdefgh");
}

#[test]
fn ipv6_body_has_no_checksum_filled() {
    let body = build_echo_request(42, b"x", true);
    assert_eq!(body[0], 128); // request v6
    // The kernel fills the ICMPv6 checksum, so we leave zeros.
    assert_eq!(&body[2..4], &[0, 0]);
}

#[test]
fn extract_reply_matches_only_ours() {
    // Craft an IPv4 packet: 20-byte IP header + echo-reply body.
    let ident = 7;
    let mut pkt = vec![
        0x45, 0, 0, 0, 0, 0, 0, 0, 64, PROTO_ICMP, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    let body = build_echo_reply(ident, b"hi", false);
    pkt.extend_from_slice(&body);
    let got = extract_reply(ETH_P_IP, &pkt, ident).unwrap();
    assert_eq!(got.payload, b"hi");
    // Wrong id -> None.
    assert!(extract_reply(ETH_P_IP, &pkt, ident + 1).is_none());
    // A request under the same packet must not match extract_reply.
    let mut pkt2 = pkt[..20].to_vec();
    pkt2.extend_from_slice(&build_echo_request(ident, b"hi", false));
    assert!(extract_reply(ETH_P_IP, &pkt2, ident).is_none());
}
