//! Session ownership: the header is attacker-chosen, so a second handshake on
//! a live session id must not be able to re-point it.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use leviculum_std::api::Identity;

use super::super::datagram::{self, Datagram, Kind};
use super::super::keyx;
use super::ServerEngine;
use crate::covert::carrier::CarrierServer;

const SESSION: u32 = 7;

#[derive(Default)]
struct Recorder {
    sent: Mutex<Vec<(IpAddr, Vec<u8>)>>,
}

impl CarrierServer for Recorder {
    type ReplyTo = IpAddr;
    fn capacity_for(&self, _reply_to: &IpAddr) -> usize {
        1024
    }
    fn send_response(&self, reply_to: &IpAddr, wire: &[u8]) -> std::io::Result<()> {
        self.sent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((*reply_to, wire.to_vec()));
        Ok(())
    }
    fn recv_request(&self, _buf: &mut [u8]) -> std::io::Result<Option<(IpAddr, Vec<u8>)>> {
        Ok(None)
    }
}

fn identity_from(seed: u8) -> Identity {
    let mut private = [0u8; 64];
    for (index, byte) in private.iter_mut().enumerate() {
        *byte = seed.wrapping_add(index as u8);
    }
    Identity::from_private_key_bytes(&private).expect("deterministic identity")
}

fn address(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(203, 0, 113, last))
}

fn handshake(server: &Identity, key: &[u8]) -> Vec<u8> {
    let token = keyx::seal(server, key).expect("seal session key");
    datagram::pack(
        &Datagram {
            session: SESSION,
            seq: 0,
            ack: 0,
            kind: Kind::Handshake,
            payload: token,
        },
        b"",
        0,
    )
}

fn engine_with(carrier: Arc<Recorder>, server: Identity) -> ServerEngine<Arc<Recorder>> {
    ServerEngine::new(carrier, server, |_session, _bytes| {}, None)
}

#[test]
fn a_handshake_carrying_a_different_key_leaves_a_live_session_alone() {
    let carrier = Arc::new(Recorder::default());
    let server = identity_from(1);
    let mut engine = engine_with(Arc::clone(&carrier), identity_from(1));

    let owner = keyx::new_session_key();
    let owner_address = address(10);
    engine
        .on_received(owner_address, &handshake(&server, &owner), 0.0)
        .expect("owner handshake accepted");

    let intruder = keyx::new_session_key();
    let intruder_address = address(66);
    engine
        .on_received(intruder_address, &handshake(&server, &intruder), 1.0)
        .expect("intruder handshake handled");

    let sent = carrier.sent.lock().unwrap_or_else(|e| e.into_inner());
    assert!(
        sent.iter().all(|(to, _)| *to == owner_address),
        "the server answered somewhere other than the owner: {:?}",
        sent.iter().map(|(to, _)| *to).collect::<Vec<_>>()
    );
    assert!(
        sent.iter()
            .all(|(_, wire)| datagram::unpack(wire, &owner, engine.tag).is_some()),
        "a downlink packet was not authenticated with the owner's key"
    );
}

#[test]
fn a_repeated_handshake_from_the_owner_still_answers() {
    let carrier = Arc::new(Recorder::default());
    let server = identity_from(2);
    let mut engine = engine_with(Arc::clone(&carrier), identity_from(2));

    let owner = keyx::new_session_key();
    for at in [0.0, 5.0] {
        engine
            .on_received(address(10), &handshake(&server, &owner), at)
            .expect("owner handshake accepted");
    }

    let answers = carrier.sent.lock().unwrap_or_else(|e| e.into_inner()).len();
    assert_eq!(answers, 2, "a retransmitted handshake went unanswered");
}
