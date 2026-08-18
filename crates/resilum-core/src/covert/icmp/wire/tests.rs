use super::*;

#[test]
fn checksum_matches_known_ping() {
    let body = build_echo_request(1234, b"abcdefgh", false);
    assert_eq!(body[0], 8); // request
    assert_eq!(u16::from_be_bytes([body[4], body[5]]), 1234);
    // Summed over the whole buffer a valid checksum comes to zero (RFC 1071).
    let mut buf = body.clone();
    let mut sum: u32 = 0;
    for c in buf.chunks_exact(2) {
        sum = sum.wrapping_add(u16::from_be_bytes([c[0], c[1]]) as u32);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    assert_eq!(sum as u16, 0xFFFF, "checksum verification failed");
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
    // A 20-byte IPv4 header, then the echo-reply body.
    let ident = 7;
    let mut pkt = vec![
        0x45, 0, 0, 0, 0, 0, 0, 0, 64, PROTO_ICMP, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    let body = build_echo_reply(ident, b"hi", false);
    pkt.extend_from_slice(&body);
    let got = extract_reply(ETH_P_IP, &pkt, ident).unwrap();
    assert_eq!(got.payload, b"hi");
    assert!(extract_reply(ETH_P_IP, &pkt, ident + 1).is_none());
    // A request rather than a reply must not match either.
    let mut pkt2 = pkt[..20].to_vec();
    pkt2.extend_from_slice(&build_echo_request(ident, b"hi", false));
    assert!(extract_reply(ETH_P_IP, &pkt2, ident).is_none());
}

#[test]
fn a_header_longer_than_the_packet_is_refused_rather_than_read() {
    let ident = 7;
    let mut pkt = vec![
        0x4F, 0, 0, 0, 0, 0, 0, 0, 64, PROTO_ICMP, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    pkt.extend_from_slice(&build_echo_reply(ident, b"hi", false));

    assert!(extract_reply(ETH_P_IP, &pkt, ident).is_none());
}

#[test]
fn a_header_length_of_zero_does_not_read_the_ip_header_as_a_payload() {
    let ident: u16 = 7;
    let mut pkt = vec![0u8; 20];
    pkt[4..6].copy_from_slice(&ident.to_be_bytes());
    pkt[9] = PROTO_ICMP;
    pkt.extend_from_slice(b"payload from inside the header");

    assert!(extract_reply(ETH_P_IP, &pkt, ident).is_none());
}
