//! The session id travels in the clear, so a second handshake on a live id
//! must not be able to re-point that session at whoever sent it.

use super::{SESSION, address, datagram, engine_of, handshake_for, keyx};

#[test]
fn a_handshake_carrying_a_different_key_leaves_a_live_session_alone() {
    let (carrier, server, mut engine) = engine_of(1);

    let owner = keyx::new_session_key();
    let owner_address = address(10);
    engine
        .on_received(owner_address, &handshake_for(SESSION, &server, &owner), 0.0)
        .expect("owner handshake accepted");

    let intruder = keyx::new_session_key();
    engine
        .on_received(
            address(66),
            &handshake_for(SESSION, &server, &intruder),
            1.0,
        )
        .expect("intruder handshake handled");

    let answered = carrier.addressees();
    assert!(
        answered.iter().all(|to| *to == owner_address),
        "the server answered somewhere other than the owner: {answered:?}"
    );
    assert!(
        carrier
            .packets()
            .iter()
            .all(|wire| datagram::unpack(wire, &owner, engine.tag).is_some()),
        "a downlink packet was not authenticated with the owner's key"
    );
}

#[test]
fn a_repeated_handshake_from_the_owner_still_answers() {
    let (carrier, server, mut engine) = engine_of(2);

    let owner = keyx::new_session_key();
    for at in [0.0, 5.0] {
        engine
            .on_received(address(10), &handshake_for(SESSION, &server, &owner), at)
            .expect("owner handshake accepted");
    }

    assert_eq!(
        carrier.packets().len(),
        2,
        "a retransmitted handshake went unanswered"
    );
}
