//! Shared rig: a carrier that records what the server sent, and packet
//! builders that let a test speak as an owner or as a stranger.

mod admission;
mod ownership;

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

impl Recorder {
    fn addressees(&self) -> Vec<IpAddr> {
        self.sent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(to, _)| *to)
            .collect()
    }

    fn packets(&self) -> Vec<Vec<u8>> {
        self.sent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(_, wire)| wire.clone())
            .collect()
    }
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

fn handshake_for(session: u32, server: &Identity, key: &[u8]) -> Vec<u8> {
    let token = keyx::seal(server, key).expect("seal session key");
    datagram::pack(
        &Datagram {
            session,
            seq: 0,
            ack: 0,
            kind: Kind::Handshake,
            payload: token,
        },
        b"",
        0,
    )
}

fn poll_packet(session: u32, key: &[u8]) -> Vec<u8> {
    datagram::pack(
        &Datagram {
            session,
            seq: 0,
            ack: 0,
            kind: Kind::Poll,
            payload: Vec::new(),
        },
        key,
        datagram::DEFAULT_TAG_LEN,
    )
}

fn engine_of(seed: u8) -> (Arc<Recorder>, Identity, ServerEngine<Arc<Recorder>>) {
    let carrier = Arc::new(Recorder::default());
    let engine = ServerEngine::new(
        Arc::clone(&carrier),
        identity_from(seed),
        |_session, _bytes| {},
        None,
    );
    (carrier, identity_from(seed), engine)
}
