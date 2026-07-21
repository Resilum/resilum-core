use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Events emitted by a node, drained by the daemon and the FFI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started,
    Stopped,
    /// A peer/anchor became reachable (destination hash).
    PeerDiscovered(Vec<u8>),
    /// Inbound data arrived.
    Received {
        source: Vec<u8>,
        data: Vec<u8>,
    },
}

/// Shared outbound queue; async tasks push, the consumer drains via `poll_event`.
pub(crate) type Queue = Arc<Mutex<VecDeque<Event>>>;

const CAP: usize = 1024;

pub(crate) fn push(queue: &Queue, event: Event) {
    let mut queue = queue.lock().expect("event queue");
    if queue.len() >= CAP {
        queue.pop_front();
    }
    queue.push_back(event);
}
