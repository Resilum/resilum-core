use proptest::prelude::*;

use super::{Arrived, Reassembly, fragment, payload_per_fragment};
use crate::ble::spec;

const WRITABLE: usize = 20;

fn reassemble(pieces: &[Vec<u8>], writable: usize) -> Option<Vec<u8>> {
    let mut held = Reassembly::default();
    let mut out = None;
    for piece in pieces {
        if let Arrived::Packet(packet) = held.process(piece, writable, 0) {
            out = Some(packet);
        }
    }
    out
}

#[test]
fn a_packet_that_fits_goes_out_as_one_start_fragment_never_lone() {
    let pieces = fragment(&[0xAA; 4], WRITABLE);

    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0][..5], [spec::TYPE_START, 0, 0, 0, 1]);
    assert_eq!(&pieces[0][5..], &[0xAA; 4]);
}

#[test]
fn a_longer_packet_runs_start_continue_end_with_the_sequence_counting_up() {
    let pieces = fragment(&[0x5A; 40], WRITABLE);

    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[0][..5], [spec::TYPE_START, 0, 0, 0, 3]);
    assert_eq!(pieces[1][..5], [spec::TYPE_CONTINUE, 0, 1, 0, 3]);
    assert_eq!(pieces[2][..5], [spec::TYPE_END, 0, 2, 0, 3]);
}

#[test]
fn a_fragment_never_exceeds_what_one_write_can_carry() {
    for writable in [spec::FRAGMENT_HEADER_LEN + 1, 20, 185, 512] {
        for piece in fragment(&[0u8; 900], writable) {
            assert!(piece.len() <= writable, "{} > {writable}", piece.len());
        }
    }
}

#[test]
fn a_lone_fragment_from_an_older_peer_is_still_accepted() {
    let mut held = Reassembly::default();
    let lone = [spec::TYPE_LONE_ACCEPTED_NEVER_SENT, 0, 0, 0, 1, 0xEE];

    assert_eq!(
        held.process(&lone, WRITABLE, 0),
        Arrived::Packet(vec![0xEE])
    );
}

#[test]
fn a_peer_claiming_more_fragments_than_a_packet_could_need_is_refused() {
    let mut held = Reassembly::default();
    let greedy = [spec::TYPE_START, 0, 0, 0xFF, 0xFF, 0x01];

    assert_eq!(held.process(&greedy, WRITABLE, 0), Arrived::Refused);
}

#[test]
fn a_reassembly_that_stalls_is_abandoned_so_the_next_packet_still_arrives() {
    let long = fragment(&[0x11; 40], WRITABLE);
    let mut held = Reassembly::default();
    held.process(&long[0], WRITABLE, 0);

    let later = spec::ABANDON_REASSEMBLY_AFTER_MS + 1;
    let fresh = fragment(&[0x22; 4], WRITABLE);

    assert_eq!(
        held.process(&fresh[0], WRITABLE, later),
        Arrived::Packet(vec![0x22; 4])
    );
}

#[test]
fn a_fragment_out_of_turn_drops_the_packet_rather_than_splicing_it() {
    let pieces = fragment(&[0x33; 40], WRITABLE);
    let mut held = Reassembly::default();
    held.process(&pieces[0], WRITABLE, 0);

    assert_eq!(held.process(&pieces[2], WRITABLE, 0), Arrived::Refused);
}

#[test]
fn a_fragment_shorter_than_its_header_is_refused() {
    let mut held = Reassembly::default();

    assert_eq!(
        held.process(&[spec::TYPE_START, 0], WRITABLE, 0),
        Arrived::Refused
    );
}

proptest! {
    #[test]
    fn any_packet_at_any_writable_size_survives_the_round_trip(
        packet in proptest::collection::vec(any::<u8>(), 1..=spec::LARGEST_PACKET),
        writable in (spec::FRAGMENT_HEADER_LEN + 1)..=512usize,
    ) {
        let pieces = fragment(&packet, writable);
        prop_assert!(!pieces.is_empty());
        prop_assert_eq!(reassemble(&pieces, writable), Some(packet));
    }

    #[test]
    fn the_payload_of_every_fragment_fills_the_room_the_size_allows(
        writable in (spec::FRAGMENT_HEADER_LEN + 1)..=512usize,
    ) {
        let room = payload_per_fragment(writable);
        let pieces = fragment(&vec![0u8; room * 3], writable);

        prop_assert_eq!(pieces.len(), 3);
        for piece in pieces {
            prop_assert_eq!(piece.len() - spec::FRAGMENT_HEADER_LEN, room);
        }
    }
}
