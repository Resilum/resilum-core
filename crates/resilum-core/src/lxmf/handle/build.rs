use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex, mpsc};

use super::{Command, EventSink, LxmfHandle, RouterState, sink};
use crate::lxmf::inbox::Inbox;

/// Build both ends of the queues at once, so the counters cannot be mismatched.
pub(in crate::lxmf) fn channel(
    address_hex: String,
    registered: Arc<AtomicBool>,
    inbox: Arc<Inbox>,
) -> (LxmfHandle, mpsc::Receiver<Command>, EventSink) {
    let (commands, command_rx) = mpsc::channel();
    let (events, event_rx) = mpsc::channel();
    let depth = Arc::new(AtomicUsize::new(0));
    let handle = LxmfHandle {
        commands,
        events: Mutex::new(event_rx),
        inbox,
        depth: depth.clone(),
        router_state: RouterState::new(),
        registered,
        address_hex,
    };
    (handle, command_rx, sink::new(events, depth))
}
