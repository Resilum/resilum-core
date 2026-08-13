//! Central node-event dispatcher: drains leviculum's single event stream and
//! fans each event out to subsystem subscribers.

use std::sync::Arc;

use leviculum_std::NodeEvent;
use leviculum_std::driver::EventReceiver;
use tokio::sync::broadcast;

/// One publisher, many subscribers. Values are shared via `Arc` because node
/// events are not `Clone`.
pub struct Fanout<T> {
    tx: broadcast::Sender<Arc<T>>,
}

// Manual impl: cloning the sender does not require `T: Clone`.
impl<T> Clone for Fanout<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T> Fanout<T> {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<T>> {
        self.tx.subscribe()
    }

    pub fn publish(&self, value: T) {
        let _ = self.tx.send(Arc::new(value)); // no subscribers is fine
    }
}

/// The node-event fan-out shared across subsystems.
pub type Events = Fanout<NodeEvent>;

/// Forward the node's event stream into `events` until the stream closes.
pub async fn forward(events: Events, mut rx: EventReceiver) {
    while let Some(event) = rx.recv().await {
        match &event {
            NodeEvent::AnnounceReceived { announce, .. } => {
                tracing::debug!(
                    id = %crate::hex::head(&announce.computed_identity_hash()),
                    dest = %crate::hex::head(announce.destination_hash().as_ref()),
                    name = %crate::hex::head(announce.name_hash()),
                    "announce"
                );
            }
            NodeEvent::LinkEstablished {
                link_id,
                destination_hash,
                is_initiator,
            } => {
                tracing::info!(?link_id, dest = %crate::hex::head(destination_hash.as_ref()), initiator = is_initiator, "link established");
            }
            _ => {}
        }
        events.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use super::Fanout;

    #[tokio::test]
    async fn fans_out_to_every_subscriber() {
        let f: Fanout<u32> = Fanout::new(8);
        let mut a = f.subscribe();
        let mut b = f.subscribe();
        f.publish(42);
        assert_eq!(*a.recv().await.unwrap(), 42);
        assert_eq!(*b.recv().await.unwrap(), 42);
    }
}
