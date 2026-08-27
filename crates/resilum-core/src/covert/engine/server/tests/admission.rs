//! What it takes to occupy a slot in the session table, and how many slots
//! there are at all.

use super::super::super::session::SESSIONS_HELD_AT_ONCE;
use super::{address, engine_of, handshake_for, keyx, poll_packet};

#[test]
fn traffic_on_an_id_nobody_has_authenticated_leaves_no_session_behind() {
    let (carrier, _server, mut engine) = engine_of(3);

    for session in 0..64u32 {
        engine
            .on_received(
                address(66),
                &poll_packet(session, b"not the session key"),
                0.0,
            )
            .expect("unauthenticated poll handled");
    }

    assert_eq!(
        engine.count(),
        0,
        "the table grew on unauthenticated traffic"
    );
    assert!(
        carrier.packets().is_empty(),
        "the server answered an unauthenticated packet"
    );
}

#[test]
fn a_full_table_refuses_new_handshakes_instead_of_growing() {
    let (_carrier, server, mut engine) = engine_of(4);

    for session in 0..(SESSIONS_HELD_AT_ONCE as u32 + 8) {
        let key = keyx::new_session_key();
        engine
            .on_received(address(66), &handshake_for(session, &server, &key), 0.0)
            .expect("handshake handled");
    }

    assert_eq!(
        engine.count(),
        SESSIONS_HELD_AT_ONCE,
        "the session table grew past its bound"
    );
}
