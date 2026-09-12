//! Received messages, held until the caller takes them.
//!
//! The router hands a message over once and keeps no copy, so the event queue
//! dropping one loses it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use resilum_store::Document;

/// Without a ceiling a flood fills the disk instead of the queue.
pub(super) const MAX_HELD: usize = 10_000;

type Held = BTreeMap<u64, String>;

pub(super) struct Inbox {
    held: Document<Held>,
    next_seq: Mutex<u64>,
    dropped: Mutex<u64>,
}

impl Inbox {
    pub(super) fn open(path: PathBuf) -> Self {
        let held = Document::open_or_start_empty(path, Held::new(), decode, encode);
        let next_seq = held.read(|held| held.keys().next_back().map_or(0, |seq| seq + 1));
        Self {
            held,
            next_seq: Mutex::new(next_seq),
            dropped: Mutex::new(0),
        }
    }

    /// For a node with no storage directory: held, but not across a restart.
    pub(super) fn ephemeral() -> Self {
        Self {
            held: Document::in_memory(Held::new()),
            next_seq: Mutex::new(0),
            dropped: Mutex::new(0),
        }
    }

    /// Runs in the engine tick, so the disk write belongs to the writer thread.
    pub(super) fn push(&self, json: String) {
        let seq = {
            let mut next = self.lock(&self.next_seq);
            let seq = *next;
            *next += 1;
            seq
        };
        let room = self.held.change(|held| {
            if held.len() >= MAX_HELD {
                return false;
            }
            held.insert(seq, json);
            true
        });
        if !room {
            *self.lock(&self.dropped) += 1;
        }
    }

    pub(super) fn pop(&self) -> Option<String> {
        self.held.change(|held| {
            let seq = *held.keys().next()?;
            held.remove(&seq)
        })
    }

    pub(super) fn take_dropped(&self) -> u64 {
        std::mem::take(&mut self.lock(&self.dropped))
    }

    fn lock<'a, T>(&self, m: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
        m.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn decode(bytes: &[u8]) -> Result<Held, String> {
    rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
}

fn encode(held: &Held) -> Vec<u8> {
    rmp_serde::to_vec(held).unwrap_or_default()
}

#[cfg(test)]
#[path = "inbox_tests.rs"]
mod tests;
