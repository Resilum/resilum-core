//! Popped events over the C ABI: an opaque handle plus byte-buffer accessors.

use std::os::raw::c_int;

use resilum_core::{Event, Node};

use crate::{
    RESILUM_EVENT_NONE, RESILUM_EVENT_PEER_DISCOVERED, RESILUM_EVENT_RECEIVED,
    RESILUM_EVENT_STARTED, RESILUM_EVENT_STOPPED, guard,
};

/// An event popped from a node. Opaque; release with `resilum_event_free`.
pub struct ResilumEvent {
    kind: c_int,
    source: Vec<u8>,
    data: Vec<u8>,
}

impl ResilumEvent {
    fn bare(kind: c_int) -> Self {
        Self {
            kind,
            source: Vec::new(),
            data: Vec::new(),
        }
    }
}

/// Pop the next queued event, or null when the queue is empty.
///
/// # Safety
/// `node` must be a live pointer from `resilum_node_new_from_yaml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_poll_event(node: *mut Node) -> *mut ResilumEvent {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_mut() }) else {
            return std::ptr::null_mut();
        };
        let event = match node.poll_event() {
            None => return std::ptr::null_mut(),
            Some(Event::Started) => ResilumEvent::bare(RESILUM_EVENT_STARTED),
            Some(Event::Stopped) => ResilumEvent::bare(RESILUM_EVENT_STOPPED),
            Some(Event::PeerDiscovered(hash)) => ResilumEvent {
                kind: RESILUM_EVENT_PEER_DISCOVERED,
                source: hash,
                data: Vec::new(),
            },
            Some(Event::Received { source, data }) => ResilumEvent {
                kind: RESILUM_EVENT_RECEIVED,
                source,
                data,
            },
        };
        Box::into_raw(Box::new(event))
    })
}

/// # Safety
/// `event` must come from `resilum_node_poll_event`, or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_event_kind(event: *const ResilumEvent) -> c_int {
    guard(RESILUM_EVENT_NONE, || {
        (unsafe { event.as_ref() }).map_or(RESILUM_EVENT_NONE, |e| e.kind)
    })
}

/// Source hash bytes (null if empty), writing the length to `len`.
///
/// # Safety
/// `event` from `resilum_node_poll_event`; `len` valid or null. Valid until free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_event_source(
    event: *const ResilumEvent,
    len: *mut usize,
) -> *const u8 {
    unsafe { buffer(event, len, |e| &e.source) }
}

/// Received data bytes (null if empty), writing the length to `len`.
///
/// # Safety
/// `event` from `resilum_node_poll_event`; `len` valid or null. Valid until free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_event_data(
    event: *const ResilumEvent,
    len: *mut usize,
) -> *const u8 {
    unsafe { buffer(event, len, |e| &e.data) }
}

unsafe fn buffer(
    event: *const ResilumEvent,
    len: *mut usize,
    pick: impl Fn(&ResilumEvent) -> &[u8],
) -> *const u8 {
    guard(std::ptr::null(), || {
        let bytes = (unsafe { event.as_ref() }).map_or(&[][..], pick);
        if !len.is_null() {
            unsafe { *len = bytes.len() };
        }
        if bytes.is_empty() {
            std::ptr::null()
        } else {
            bytes.as_ptr()
        }
    })
}

/// # Safety
/// `event` must come from `resilum_node_poll_event` and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_event_free(event: *mut ResilumEvent) {
    if !event.is_null() {
        drop(unsafe { Box::from_raw(event) });
    }
}
