//! The processor's end of the stamp thread: what to grind, and what is already
//! being ground.

use std::collections::HashMap;

use leviculum_lxmf::{DeliveryStampRequest, PropagationStampRequest};
use tokio::sync::mpsc::UnboundedSender;

use super::Job;

pub(in crate::lxmf) struct Jobs {
    tx: UnboundedSender<Job>,
    /// What each message is currently being mined a propagation stamp for.
    ///
    /// The router re-asks every processing interval until the stamp lands, and
    /// one grind outlives many of those repeats. Without this the queue would
    /// take a duplicate per repeat, and every stamp behind them — including the
    /// delivery stamps sharing the thread — would wait out the whole pile.
    mining: HashMap<[u8; 32], PropagationStampRequest>,
}

impl Jobs {
    pub(in crate::lxmf) fn new(tx: UnboundedSender<Job>) -> Self {
        Self {
            tx,
            mining: HashMap::new(),
        }
    }

    pub(in crate::lxmf) fn delivery(&self, request: DeliveryStampRequest) {
        self.send(Job::Delivery(request));
    }

    /// A repeat of what the thread already holds is dropped. A request that
    /// differs is not a repeat: the entry was re-prepared for another
    /// propagation node, and the grind in flight can no longer be applied to
    /// it.
    pub(in crate::lxmf) fn propagation(&mut self, request: PropagationStampRequest) {
        if self.mining.get(&request.message_id) == Some(&request) {
            return;
        }
        if self.send(Job::Propagation(request)) {
            self.mining.insert(request.message_id, request);
        }
    }

    /// The thread has answered, so the next ask for this message is work again
    /// rather than a repeat.
    pub(in crate::lxmf) fn finished(&mut self, request: &PropagationStampRequest) {
        if self.mining.get(&request.message_id) == Some(request) {
            self.mining.remove(&request.message_id);
        }
    }

    fn send(&self, job: Job) -> bool {
        let sent = self.tx.send(job).is_ok();
        if !sent {
            tracing::error!("lxmf stamp thread is gone; priced messages cannot be sent");
        }
        sent
    }
}
