//! The publish round driven through the entry points the loop calls, with
//! only the relays and the mesh sender standing in.

mod expiry;
mod queued;
mod rounds;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use data_encoding::BASE64;
use serde_json::Value;

use super::{Pending, Publishing, Source, Verdict};
use crate::bridge::from_mesh::ack_json;
use crate::bridge::retry::Schedule;
use crate::bridge::state;
use crate::config::NostrConfig;
use crate::event::Event;
use crate::queue::{Direction, Entry, Handoff, Queue};
use crate::registry::Registry;

const RETENTION: Duration = Duration::from_secs(600);
const PER_SUBSCRIBER: usize = 16;
const PEER_A: [u8; 16] = [0xaa; 16];
const PEER_B: [u8; 16] = [0xbb; 16];
const REFUSED: &str = "blocked";

/// The stores are the real ones; only the two ways out stand in.
struct Held {
    cfg: NostrConfig,
    registry: Registry,
    queue: Queue,
    retry: Schedule,
    relays: usize,
    broadcasts: Mutex<Vec<String>>,
    acks: Mutex<Vec<([u8; 16], Value)>>,
}

fn held(relays: usize) -> Held {
    Held {
        cfg: NostrConfig::default(),
        registry: Registry::ephemeral(RETENTION),
        queue: Queue::ephemeral(RETENTION, PER_SUBSCRIBER),
        retry: Schedule::default(),
        relays,
        broadcasts: Mutex::new(Vec::new()),
        acks: Mutex::new(Vec::new()),
    }
}

impl Held {
    fn publishing(&self) -> Publishing<'_> {
        Publishing {
            cfg: &self.cfg,
            registry: &self.registry,
            queue: &self.queue,
            retry: &self.retry,
            broadcast: Box::new(|frame| {
                self.broadcasts
                    .lock()
                    .expect("nothing panics holding it")
                    .push(frame.to_owned());
                self.relays
            }),
            ack: Box::new(|dest, event_id, verdicts| {
                self.acks
                    .lock()
                    .expect("nothing panics holding it")
                    .push((dest, ack_json(event_id, verdicts)));
            }),
        }
    }

    fn broadcasts_of(&self, event_id: &str) -> usize {
        self.broadcasts
            .lock()
            .expect("nothing panics holding it")
            .iter()
            .filter(|frame| frame.contains(event_id))
            .count()
    }

    fn acked_at(&self, dest: [u8; 16]) -> Vec<Value> {
        self.acks
            .lock()
            .expect("nothing panics holding it")
            .iter()
            .filter(|(to, _)| *to == dest)
            .map(|(_, ack)| ack.clone())
            .collect()
    }
}

/// A signed gift wrap and the mesh request carrying it, as a peer sends one.
fn published(content: &str) -> (Event, Value) {
    let event = crate::signed::gift_wrap([7u8; 32], 1_786_733_083, content);
    let json = serde_json::to_string(&event).expect("re-encodes");
    (event, Value::String(BASE64.encode(json.as_bytes())))
}

/// The same event as the queue holds it, waiting on a relay that will have
/// it, with `PEER_B` as the address its retry is owed to.
fn held_entry(event: &Event) -> Entry {
    Entry {
        direction: Direction::Outbound,
        subscriber: event.pubkey_bytes().expect("a signed event's key"),
        lxmf: PEER_B,
        event_id: event.id_bytes().expect("a signed event's id"),
        event_json: serde_json::to_string(event).expect("re-encodes").into(),
        queued_at: state::now(),
        handoff: Handoff::default(),
    }
}

/// A peer at `peer` forwarding `request`, as the loop hands one over.
fn offer(held: &Held, pending: &mut Pending, request: &Value, peer: [u8; 16]) {
    pending.offer(
        &held.publishing(),
        request,
        Source::Recalled(peer),
        Instant::now(),
    );
}

/// Every relay answers every copy of the event it was sent, as NIP-01 has it
/// answer each `EVENT` with one `OK`.
fn every_relay_answers(held: &Held, pending: &mut Pending, event: &Event, relays: &[bool]) {
    let copies = held.broadcasts_of(&event.id);
    for accepted in relays {
        for _ in 0..copies {
            pending.verdict(&held.publishing(), &event.id, verdict(*accepted));
        }
    }
}

fn verdict(accepted: bool) -> Verdict {
    if accepted {
        Verdict::Accepted
    } else {
        Verdict::Rejected(REFUSED.to_owned())
    }
}
