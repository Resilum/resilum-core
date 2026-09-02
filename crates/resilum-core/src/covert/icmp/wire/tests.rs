use super::*;

const M: [u8; MARKER_LEN] = [0xDE, 0xAD, 0xBE, 0xEF];

#[test]
fn checksum_matches_known_ping() {
    let body = build_echo_request(M, b"abcdefgh", false);
    assert_eq!(body[0], 8);

    let mut sum: u32 = 0;
    for c in body.as_chunks::<2>().0 {
        sum = sum.wrapping_add(u16::from_be_bytes([c[0], c[1]]) as u32);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    assert_eq!(sum as u16, 0xFFFF, "checksum verification failed");

    assert_eq!(&body[8..12], &M);
    assert_eq!(&body[12..], b"abcdefgh");
}

#[test]
fn ipv6_body_has_no_checksum_filled() {
    let body = build_echo_request(M, b"x", true);
    assert_eq!(body[0], 128);
    assert_eq!(&body[2..4], &[0, 0]);
}

fn ipv4_packet(body: &[u8]) -> Vec<u8> {
    let mut pkt = vec![
        0x45, 0, 0, 0, 0, 0, 0, 0, 64, PROTO_ICMP, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    pkt.extend_from_slice(body);
    pkt
}

#[test]
fn a_request_is_recognised_by_its_marker_and_yields_the_clients_id() {
    let mut body = build_echo_request(M, b"hi", false);
    body[4..6].copy_from_slice(&0x1234u16.to_be_bytes());
    let pkt = ipv4_packet(&body);

    let got = extract_request(ETH_P_IP, &pkt, M).expect("marked request");
    assert_eq!(got.payload, b"hi");
    assert_eq!(got.id, 0x1234);
}

#[test]
fn another_servers_marker_does_not_match() {
    let pkt = ipv4_packet(&build_echo_request(M, b"hi", false));
    assert!(extract_request(ETH_P_IP, &pkt, [1, 2, 3, 4]).is_none());
}

#[test]
fn a_reply_is_not_read_as_a_request() {
    let pkt = ipv4_packet(&build_echo_reply(9, M, b"hi", false));
    assert!(extract_request(ETH_P_IP, &pkt, M).is_none());
}

#[test]
fn a_reply_the_client_gets_keeps_the_data_behind_the_marker() {
    let body = build_echo_reply(9, M, b"pong", false);
    assert_eq!(payload_of_reply(&body, M, false), Some(&b"pong"[..]));
    assert!(payload_of_reply(&body, [0, 0, 0, 0], false).is_none());
}

const REQUEST_M: [u8; MARKER_LEN] = [1, 2, 3, 4];
const REPLY_M: [u8; MARKER_LEN] = [5, 6, 7, 8];

#[test]
fn a_kernel_echo_of_our_request_is_not_taken_for_a_reply() {
    let request = build_echo_request(REQUEST_M, b"data", false);
    let reflected = kernel_echo_of(&request);

    assert!(payload_of_reply(&reflected, REPLY_M, false).is_none());
}

fn kernel_echo_of(request: &[u8]) -> Vec<u8> {
    let mut echo = request.to_vec();
    echo[0] = REPLY_V4;
    echo
}

#[test]
fn a_header_longer_than_the_packet_is_refused_rather_than_read() {
    let mut pkt = vec![
        0x4F, 0, 0, 0, 0, 0, 0, 0, 64, PROTO_ICMP, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    pkt.extend_from_slice(&build_echo_request(M, b"hi", false));

    assert!(extract_request(ETH_P_IP, &pkt, M).is_none());
}

#[test]
fn a_header_length_of_zero_does_not_read_the_ip_header_as_a_payload() {
    let mut pkt = vec![0u8; 20];
    pkt[9] = PROTO_ICMP;
    pkt.extend_from_slice(b"payload from inside the header");

    assert!(extract_request(ETH_P_IP, &pkt, M).is_none());
}
