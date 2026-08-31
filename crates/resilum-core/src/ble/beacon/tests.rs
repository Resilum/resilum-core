use proptest::prelude::*;

use super::Beacon;

const ROOM_IN_THE_ADVERTISEMENT: usize = 8;

fn beacon(tiebreak: [u8; 3], can_host: bool, group_is_up: bool) -> Beacon {
    Beacon {
        tiebreak,
        can_host,
        group_is_up,
    }
}

#[test]
fn a_name_fits_what_the_advertisement_has_room_for() {
    let widest = beacon([0xFF; 3], true, true);

    assert!(
        widest.name().len() <= ROOM_IN_THE_ADVERTISEMENT,
        "{} chars",
        widest.name().len()
    );
}

#[test]
fn a_name_carries_both_switches_back() {
    for (can_host, group_is_up) in [(false, false), (true, false), (false, true), (true, true)] {
        let ours = beacon([1, 2, 3], can_host, group_is_up);

        assert_eq!(Beacon::read(&ours.name()), Some(ours));
    }
}

#[test]
fn a_peer_that_is_not_one_of_ours_is_not_read_as_one() {
    for name in ["", "Columba", "LN-0a1b2c3d", "R", "R!!!!!!", "Rzzzzzzz"] {
        assert_eq!(Beacon::read(name), None, "{name} was taken for ours");
    }
}

#[test]
fn two_nodes_advertising_at_once_do_not_share_a_tiebreak() {
    let ours = Beacon::fresh(true, false);
    let theirs = Beacon::fresh(true, false);

    assert_ne!(ours.tiebreak, theirs.tiebreak);
}

#[test]
fn a_flag_we_do_not_know_yet_does_not_spoil_the_ones_we_do() {
    let known = beacon([9, 9, 9], true, true);
    let name = known.name();
    let body = name.strip_prefix('R').expect("ours");
    let mut wider = data_encoding::BASE32_NOPAD
        .decode(body.as_bytes())
        .expect("ours");
    wider[3] |= 0b1000_0000;
    let from_the_future = format!("R{}", data_encoding::BASE32_NOPAD.encode(&wider));

    assert_eq!(Beacon::read(&from_the_future), Some(known));
}

proptest! {
    #[test]
    fn any_beacon_survives_the_name_it_is_written_into(
        tiebreak in any::<[u8; 3]>(),
        can_host in any::<bool>(),
        group_is_up in any::<bool>(),
    ) {
        let ours = beacon(tiebreak, can_host, group_is_up);

        prop_assert!(ours.name().len() <= ROOM_IN_THE_ADVERTISEMENT);
        prop_assert_eq!(Beacon::read(&ours.name()), Some(ours));
    }

    #[test]
    fn no_name_at_all_makes_this_panic(name in ".{0,40}") {
        let _ = Beacon::read(&name);
    }
}
