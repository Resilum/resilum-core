//! Demultiplexes leviculum link events onto per-session channels keyed by
//! `LinkId`. Responder-side links this node did not open are surfaced so the
//! listen side can accept and service them.

mod route;

use std::collections::HashMap;
use std::sync::Mutex;

use leviculum_std::api::{DestinationHash, LinkId};
use tokio::sync::mpsc;

pub use route::run;

/// A message routed to one link session, in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkMsg {
    Established,
    Data(Vec<u8>),
    Closed,
}

/// Routing table: one inbound-byte channel per attached link.
#[derive(Default)]
pub struct LinkRouter {
    sessions: Mutex<HashMap<LinkId, mpsc::UnboundedSender<LinkMsg>>>,
}

impl LinkRouter {
    /// Attach a session for a link this node opened. Call before awaiting
    /// establishment so the `Established` event is not missed.
    pub fn attach(&self, link_id: LinkId) -> mpsc::UnboundedReceiver<LinkMsg> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.sessions.lock().unwrap().insert(link_id, tx);
        rx
    }

    pub fn detach(&self, link_id: &LinkId) {
        self.sessions.lock().unwrap().remove(link_id);
    }

    /// Deliver to an attached session; `false` if none is attached.
    fn deliver(&self, link_id: &LinkId, msg: LinkMsg) -> bool {
        match self.sessions.lock().unwrap().get(link_id) {
            Some(tx) => tx.send(msg).is_ok(),
            None => false,
        }
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.sessions.lock().unwrap().is_empty()
    }
}

/// A responder-side link: its id, the local destination it targeted (so the
/// listen side can pick the matching service), and the attached byte channel.
pub type Inbound = (LinkId, DestinationHash, mpsc::UnboundedReceiver<LinkMsg>);
