//! Demultiplexes leviculum link events onto per-session channels keyed by
//! `LinkId`. Responder-side links this node did not open are surfaced so the
//! listen side can accept and service them.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use leviculum_std::NodeEvent;
use leviculum_std::api::LinkId;
use tokio::sync::{broadcast, mpsc};

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
}

/// Drain the node-event bus onto sessions until it closes. Established links
/// with no attached session are reported on `inbound` when this node did not
/// initiate them (responder side).
pub async fn run(
    router: Arc<LinkRouter>,
    mut bus: broadcast::Receiver<Arc<NodeEvent>>,
    inbound: mpsc::UnboundedSender<LinkId>,
) {
    loop {
        match bus.recv().await {
            Ok(ev) => route(&router, &inbound, &ev),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

fn route(router: &LinkRouter, inbound: &mpsc::UnboundedSender<LinkId>, ev: &NodeEvent) {
    match ev {
        NodeEvent::LinkEstablished {
            link_id,
            is_initiator,
        } => {
            if !router.deliver(link_id, LinkMsg::Established) && !is_initiator {
                let _ = inbound.send(*link_id);
            }
        }
        NodeEvent::LinkDataReceived { link_id, data } => {
            router.deliver(link_id, LinkMsg::Data(data.clone()));
        }
        NodeEvent::LinkClosed { link_id, .. } => {
            router.deliver(link_id, LinkMsg::Closed);
            router.detach(link_id);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lid(b: u8) -> LinkId {
        LinkId::new([b; 16])
    }

    fn established(link_id: LinkId, is_initiator: bool) -> NodeEvent {
        NodeEvent::LinkEstablished {
            link_id,
            is_initiator,
        }
    }

    #[test]
    fn routes_established_and_data_to_attached_session() {
        let router = LinkRouter::default();
        let mut rx = router.attach(lid(1));
        let (itx, mut irx) = mpsc::unbounded_channel();

        route(&router, &itx, &established(lid(1), true));
        route(
            &router,
            &itx,
            &NodeEvent::LinkDataReceived {
                link_id: lid(1),
                data: b"hi".to_vec(),
            },
        );

        assert_eq!(rx.try_recv().unwrap(), LinkMsg::Established);
        assert_eq!(rx.try_recv().unwrap(), LinkMsg::Data(b"hi".to_vec()));
        assert!(irx.try_recv().is_err(), "attached link is not surfaced");
    }

    #[test]
    fn surfaces_unclaimed_responder_link() {
        let router = LinkRouter::default();
        let (itx, mut irx) = mpsc::unbounded_channel();

        route(&router, &itx, &established(lid(2), false));

        assert_eq!(irx.try_recv().unwrap(), lid(2));
    }

    #[test]
    fn detach_removes_the_session() {
        let router = LinkRouter::default();
        let _rx = router.attach(lid(3));
        router.detach(&lid(3));
        assert!(router.sessions.lock().unwrap().is_empty());
    }
}
