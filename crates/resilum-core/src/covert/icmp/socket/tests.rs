use super::HowTheKernelHandsItOver;

const AN_ECHO_REPLY: [u8; 4] = [0, 0, 0xab, 0xcd];

fn behind_a_header_of(words: u8) -> Vec<u8> {
    let mut packet = vec![0u8; usize::from(words) * 4];
    packet[0] = 0x40 | words;
    packet.extend_from_slice(&AN_ECHO_REPLY);
    packet
}

#[test]
fn a_datagram_socket_hands_over_the_icmp_message_itself() {
    let arrived = HowTheKernelHandsItOver::StartingAtTheIcmpHeader.icmp_within(&AN_ECHO_REPLY);

    assert_eq!(arrived, AN_ECHO_REPLY);
}

#[test]
fn a_raw_socket_hands_over_the_ip_header_first() {
    let packet = behind_a_header_of(5);

    let arrived = HowTheKernelHandsItOver::BehindTheIpv4Header.icmp_within(&packet);

    assert_eq!(arrived, AN_ECHO_REPLY);
}

#[test]
fn a_header_carrying_options_is_measured_and_not_assumed() {
    let packet = behind_a_header_of(9);

    let arrived = HowTheKernelHandsItOver::BehindTheIpv4Header.icmp_within(&packet);

    assert_eq!(arrived, AN_ECHO_REPLY);
}

#[test]
fn a_packet_shorter_than_the_header_it_claims_yields_nothing() {
    let mut truncated = behind_a_header_of(9);
    truncated.truncate(8);

    let arrived = HowTheKernelHandsItOver::BehindTheIpv4Header.icmp_within(&truncated);

    assert!(arrived.is_empty());
}
