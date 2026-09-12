mod stub;

use super::{Handshakes, Told};
use crate::ble::radio::{ConnectionId, PeerAddress, Role};
use crate::ble::spec;
use stub::Stub;

const CONN: ConnectionId = ConnectionId(1);
const OURS: [u8; spec::IDENTITY_LEN] = [0xA1; spec::IDENTITY_LEN];
const THEIRS: [u8; spec::IDENTITY_LEN] = [0xB2; spec::IDENTITY_LEN];

fn begun(role: Role) -> (Stub, Handshakes) {
    let radio = Stub::default();
    let mut waiting = Handshakes::default();
    waiting
        .began(&radio, CONN, PeerAddress("aa".into()), role, 0)
        .expect("begin");
    (radio, waiting)
}

#[test]
fn a_central_asks_the_peripheral_for_its_identity_before_anything_else() {
    let (radio, _waiting) = begun(Role::Central);

    let asked = radio.asked_to_read.lock().expect("lock").clone();
    assert_eq!(asked, vec![spec::IDENTITY_READ_FROM_THE_PERIPHERAL]);
}

#[test]
fn a_peripheral_asks_for_nothing_and_waits_to_be_told() {
    let (radio, waiting) = begun(Role::Peripheral);

    assert!(radio.asked_to_read.lock().expect("lock").is_empty());
    assert!(waiting.is_waiting(CONN));
}

#[test]
fn a_central_answers_the_identity_it_read_with_its_own() {
    let (radio, mut waiting) = begun(Role::Central);

    let told = waiting.heard(
        &radio,
        CONN,
        spec::IDENTITY_READ_FROM_THE_PERIPHERAL,
        &THEIRS,
        OURS,
    );

    assert!(matches!(told, Told::ThisIsTheirIdentity(peer, _) if peer == THEIRS));
    assert_eq!(radio.what_went_out(), vec![OURS.to_vec()]);
}

#[test]
fn a_peripheral_takes_the_first_write_as_the_identity_and_answers_nothing() {
    let (radio, mut waiting) = begun(Role::Peripheral);

    let told = waiting.heard(&radio, CONN, spec::RX_WRITTEN_BY_THE_CENTRAL, &THEIRS, OURS);

    assert!(matches!(told, Told::ThisIsTheirIdentity(peer, _) if peer == THEIRS));
    assert!(radio.what_went_out().is_empty());
}

#[test]
fn a_first_write_of_the_wrong_length_is_not_taken_for_an_identity() {
    let (radio, mut waiting) = begun(Role::Peripheral);

    let told = waiting.heard(
        &radio,
        CONN,
        spec::RX_WRITTEN_BY_THE_CENTRAL,
        &[spec::TYPE_START, 0, 0, 0, 1, 0xFF],
        OURS,
    );

    assert!(matches!(told, Told::NotOneOfOurs));
}

#[test]
fn a_fragment_arriving_before_the_identity_is_not_mistaken_for_one() {
    let (radio, mut waiting) = begun(Role::Central);
    let looks_like_a_fragment = [spec::TYPE_START; spec::IDENTITY_LEN];

    let told = waiting.heard(
        &radio,
        CONN,
        spec::TX_NOTIFIED_BY_THE_PERIPHERAL,
        &looks_like_a_fragment,
        OURS,
    );

    assert!(matches!(told, Told::NotYet));
    assert!(waiting.is_waiting(CONN));
}

#[test]
fn a_handshake_that_never_finishes_is_given_up_on() {
    let (_radio, mut waiting) = begun(Role::Peripheral);

    assert_eq!(waiting.gave_up_by(9_000, 5_000), vec![CONN]);
    assert!(!waiting.is_waiting(CONN));
}
