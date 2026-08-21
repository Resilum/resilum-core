use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::Inbound;

#[derive(Default)]
pub struct Inbox {
    claimed: Mutex<HashMap<[u8; 16], UnboundedSender<Inbound>>>,
}

impl Inbox {
    pub fn claim(&self, destination: [u8; 16]) -> UnboundedReceiver<Inbound> {
        let (tx, rx) = unbounded_channel();
        self.lock().insert(destination, tx);
        rx
    }

    pub async fn sort(
        &self,
        mut arriving: UnboundedReceiver<Inbound>,
        unclaimed: UnboundedSender<Inbound>,
    ) {
        while let Some(arrival) = arriving.recv().await {
            let (_, destination, _) = &arrival;
            let claimant = self.lock().get(destination.as_bytes()).cloned();
            let delivered = match claimant {
                Some(to) => to.send(arrival),
                None => unclaimed.send(arrival),
            };
            if delivered.is_err() {
                tracing::debug!("an inbound link arrived for a listener that had gone");
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 16], UnboundedSender<Inbound>>> {
        self.claimed.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests;
