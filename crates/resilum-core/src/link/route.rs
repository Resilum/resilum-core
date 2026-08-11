//! Turning the node-event bus into per-session traffic.

use std::sync::Arc;

use leviculum_std::NodeEvent;
use tokio::sync::{broadcast, mpsc};

use super::{Inbound, LinkMsg, LinkRouter};

/// Drain the node-event bus onto sessions until it closes. Responder-side links
/// are attached here, before reporting on `inbound`, so no data event can slip
/// in before the listen side owns the session.
pub async fn run(
    router: Arc<LinkRouter>,
    mut bus: broadcast::Receiver<Arc<NodeEvent>>,
    inbound: mpsc::UnboundedSender<Inbound>,
) {
    loop {
        match bus.recv().await {
            Ok(ev) => route(&router, &inbound, &ev),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

fn route(router: &LinkRouter, inbound: &mpsc::UnboundedSender<Inbound>, ev: &NodeEvent) {
    match ev {
        NodeEvent::LinkEstablished {
            link_id,
            is_initiator,
            destination_hash,
        } => {
            if !router.deliver(link_id, LinkMsg::Established) && !is_initiator {
                let _ = inbound.send((*link_id, *destination_hash, router.attach(*link_id)));
            }
        }
        // reliable channel stream; raw LinkDataReceived is a separate path
        NodeEvent::MessageReceived { link_id, data, .. } => {
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
    use leviculum_std::api::{DestinationHash, LinkId};

    fn lid(b: u8) -> LinkId {
        LinkId::new([b; 16])
    }

    fn established(link_id: LinkId, is_initiator: bool) -> NodeEvent {
        NodeEvent::LinkEstablished {
            link_id,
            is_initiator,
            destination_hash: DestinationHash::new([7; 16]),
        }
    }

    fn message(link_id: LinkId, data: &[u8]) -> NodeEvent {
        NodeEvent::MessageReceived {
            link_id,
            msgtype: 0,
            sequence: 0,
            data: data.to_vec(),
        }
    }

    #[test]
    fn routes_established_and_data_to_attached_session() {
        let router = LinkRouter::default();
        let mut rx = router.attach(lid(1));
        let (itx, mut irx) = mpsc::unbounded_channel();

        route(&router, &itx, &established(lid(1), true));
        route(&router, &itx, &message(lid(1), b"hi"));

        assert_eq!(rx.try_recv().unwrap(), LinkMsg::Established);
        assert_eq!(rx.try_recv().unwrap(), LinkMsg::Data(b"hi".to_vec()));
        assert!(irx.try_recv().is_err(), "attached link is not surfaced");
    }

    #[test]
    fn surfaces_and_attaches_responder_link() {
        let router = LinkRouter::default();
        let (itx, mut irx) = mpsc::unbounded_channel();

        route(&router, &itx, &established(lid(2), false));
        let (id, dest, mut rx) = irx.try_recv().unwrap();
        assert_eq!(id, lid(2));
        assert_eq!(dest, DestinationHash::new([7; 16]));

        route(&router, &itx, &message(lid(2), b"x"));
        assert_eq!(rx.try_recv().unwrap(), LinkMsg::Data(b"x".to_vec()));
    }

    #[test]
    fn detach_removes_the_session() {
        let router = LinkRouter::default();
        let _rx = router.attach(lid(3));
        router.detach(&lid(3));
        assert!(router.is_empty());
    }
}
