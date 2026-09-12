//! Upstream connection to a Nostr relay: one websocket, reconnected forever.
//!
//! `Upstream::pair` splits into a cheap, cloneable handle (`send`, `is_up`)
//! and a non-`Clone` `UpstreamRunner` that owns the socket and dials. Splitting
//! the types makes a second `run` for the same connection unexpressible rather
//! than merely discouraged: the runner holds the receiving end of the outbox,
//! and an mpsc receiver has exactly one owner, so there is nothing for a second
//! runner to be built out of.
//!
//! `run`'s `on_connect` callback returns every frame to reissue after a
//! drop, so `UpstreamRunner` never tracks what it was subscribed to — the
//! caller's own state is the only copy, asked fresh on every reconnect.

pub mod proto;
mod reconnect;
mod tls;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;

pub use reconnect::UpstreamRunner;

pub(crate) use reconnect::Deadlines;

#[derive(Clone)]
pub struct Upstream {
    outbox_tx: mpsc::UnboundedSender<Utf8Bytes>,
    up: Arc<AtomicBool>,
}

impl Upstream {
    /// Nothing is dialled here: the runner does that when it is spawned.
    pub fn pair(
        url: String,
        incoming: mpsc::UnboundedSender<proto::Incoming>,
        deadlines: Deadlines,
    ) -> (Self, UpstreamRunner) {
        let (outbox_tx, outbox_rx) = mpsc::unbounded_channel();
        let up = Arc::new(AtomicBool::new(false));
        let handle = Self {
            outbox_tx,
            up: Arc::clone(&up),
        };
        let runner = UpstreamRunner::new(url, incoming, outbox_rx, up, deadlines);
        (handle, runner)
    }

    /// Frames sent while disconnected are dropped, not buffered: both
    /// callers re-send on reconnect, so a buffer here would be a second,
    /// silent source of truth about what upstream should hear.
    pub fn send(&self, frame: Utf8Bytes) {
        if self.is_up() {
            let _ = self.outbox_tx.send(frame);
        }
    }

    pub fn is_up(&self) -> bool {
        self.up.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests;
