//! The application side of the LXMF stack. The router runs inside the engine's
//! tick, so everything here reaches it over queues.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use leviculum_lxmf::Message;

use crate::error::{Error, Result};

/// Capped because a backgrounded app stops polling while the mesh keeps
/// delivering: the loss is counted and reported as an `overflow` event rather
/// than growing until the process is killed.
const MAX_QUEUED_EVENTS: usize = 4096;

pub(super) enum Command {
    /// Built and signed off the core lock, so the hook only has to accept it.
    Send(Box<Message>),
    Announce,
    /// Mined, or given up on, by the stamp thread.
    Stamp(super::stamp::Outcome),
}

pub struct LxmfHandle {
    commands: mpsc::Sender<Command>,
    events: Mutex<mpsc::Receiver<String>>,
    inbox: Arc<super::inbox::Inbox>,
    depth: Arc<AtomicUsize>,
    registered: Arc<AtomicBool>,
    address: String,
}

impl LxmfHandle {
    /// Hex, and derived from the identity alone — valid before the router has
    /// registered.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// False for the first tick after start, and permanently once the driver
    /// has detached a panicking processor.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.registered.load(Ordering::Relaxed)
    }

    /// Returns once queued, not once sent; delivery arrives through
    /// [`Self::next_event`].
    pub fn submit(&self, message: Message) -> Result<()> {
        self.send(Command::Send(Box::new(message)))
    }

    /// Worth calling on a network change: a peer that has not seen our announce
    /// cannot message us.
    pub fn announce(&self) -> Result<()> {
        self.send(Command::Announce)
    }

    /// Next queued event, as the JSON the FFI hands the app. Received messages
    /// go first; a delivery update can wait behind them.
    ///
    /// The poisoned lock is recovered from: it is only ever held across a
    /// `try_recv`, and `None` would claim the mesh went quiet.
    pub fn next_event(&self) -> Option<String> {
        if let Some(json) = self.inbox.pop() {
            return Some(json);
        }
        let refused = self.inbox.take_dropped();
        if refused > 0 {
            return Some(format!(r#"{{"type":"overflow","dropped":{refused}}}"#));
        }
        let events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        let json = events.try_recv().ok()?;
        self.depth.fetch_sub(1, Ordering::Relaxed);
        Some(json)
    }

    pub(super) fn sender(&self) -> mpsc::Sender<Command> {
        self.commands.clone()
    }

    fn send(&self, command: Command) -> Result<()> {
        self.commands
            .send(command)
            .map_err(|_| Error::Engine("lxmf processor is gone".into()))
    }
}

/// The processor's end: every push is non-blocking, which is what lets it run
/// under the core lock.
pub(super) struct EventSink {
    events: mpsc::Sender<String>,
    depth: Arc<AtomicUsize>,
    dropped: u64,
}

impl EventSink {
    pub(super) fn push(&mut self, json: String) {
        if self.depth.load(Ordering::Relaxed) >= MAX_QUEUED_EVENTS {
            self.dropped += 1;
            return;
        }
        if self.dropped > 0 {
            let lost = std::mem::take(&mut self.dropped);
            self.emit(format!(r#"{{"type":"overflow","dropped":{lost}}}"#));
        }
        self.emit(json);
    }

    fn emit(&self, json: String) {
        if self.events.send(json).is_ok() {
            self.depth.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Build both ends of the queues at once, so the counters cannot be mismatched.
pub(super) fn channel(
    address: String,
    registered: Arc<AtomicBool>,
    inbox: Arc<super::inbox::Inbox>,
) -> (LxmfHandle, mpsc::Receiver<Command>, EventSink) {
    let (commands, command_rx) = mpsc::channel();
    let (events, event_rx) = mpsc::channel();
    let depth = Arc::new(AtomicUsize::new(0));
    let handle = LxmfHandle {
        commands,
        events: Mutex::new(event_rx),
        inbox,
        depth: depth.clone(),
        registered,
        address,
    };
    let sink = EventSink {
        events,
        depth,
        dropped: 0,
    };
    (handle, command_rx, sink)
}
