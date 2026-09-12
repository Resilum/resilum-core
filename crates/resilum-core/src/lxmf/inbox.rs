//! Received messages, held until the caller takes them.
//!
//! The router hands a message over once and keeps no copy, so the event queue
//! dropping one loses it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::Sender;

mod store;

/// Without a ceiling a flood fills the disk instead of the queue.
pub(super) const MAX_HELD: usize = 10_000;

pub(super) struct Inbox {
    held: Mutex<BTreeMap<u64, String>>,
    writer: Sender<store::Change>,
    next_seq: Mutex<u64>,
    dropped: Mutex<u64>,
}

impl Inbox {
    pub(super) fn open(path: PathBuf) -> Self {
        let held = store::read(&path);
        let next_seq = held.keys().next_back().map_or(0, |seq| seq + 1);
        Self {
            writer: store::spawn_writer(path, held.clone()),
            held: Mutex::new(held),
            next_seq: Mutex::new(next_seq),
            dropped: Mutex::new(0),
        }
    }

    /// For a node with no storage directory: held, but not across a restart.
    pub(super) fn ephemeral() -> Self {
        Self {
            held: Mutex::new(BTreeMap::new()),
            writer: store::null_writer(),
            next_seq: Mutex::new(0),
            dropped: Mutex::new(0),
        }
    }

    /// Runs in the engine tick, so the disk write belongs to the writer thread.
    pub(super) fn push(&self, json: String) {
        let mut held = self.lock(&self.held);
        if held.len() >= MAX_HELD {
            *self.lock(&self.dropped) += 1;
            return;
        }
        let seq = {
            let mut next = self.lock(&self.next_seq);
            let seq = *next;
            *next += 1;
            seq
        };
        held.insert(seq, json.clone());
        let _ = self.writer.send((seq, Some(json)));
    }

    pub(super) fn pop(&self) -> Option<String> {
        let mut held = self.lock(&self.held);
        let seq = *held.keys().next()?;
        let json = held.remove(&seq)?;
        let _ = self.writer.send((seq, None));
        Some(json)
    }

    pub(super) fn take_dropped(&self) -> u64 {
        std::mem::take(&mut self.lock(&self.dropped))
    }

    fn lock<'a, T>(&self, m: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
        m.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("a temporary directory")
    }

    #[test]
    fn messages_leave_in_the_order_they_arrived() {
        let inbox = Inbox::ephemeral();
        inbox.push("first".into());
        inbox.push("second".into());

        assert_eq!(inbox.pop().as_deref(), Some("first"));
        assert_eq!(inbox.pop().as_deref(), Some("second"));
        assert_eq!(inbox.pop(), None);
    }

    #[test]
    fn a_message_outlives_the_process_that_received_it() {
        let dir = temp_dir();
        let path = dir.path().join("inbox");

        let inbox = Inbox::open(path.clone());
        inbox.push("kept".into());
        drop(inbox);
        // The writer thread owns the file; give it the moment it needs.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let reopened = Inbox::open(path);
        assert_eq!(reopened.pop().as_deref(), Some("kept"));
    }

    #[test]
    fn a_taken_message_does_not_come_back_after_a_restart() {
        let dir = temp_dir();
        let path = dir.path().join("inbox");

        let inbox = Inbox::open(path.clone());
        inbox.push("taken".into());
        assert_eq!(inbox.pop().as_deref(), Some("taken"));
        drop(inbox);
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert_eq!(Inbox::open(path).pop(), None);
    }

    #[test]
    fn refusals_past_the_ceiling_are_counted_once() {
        let inbox = Inbox::ephemeral();
        for i in 0..MAX_HELD + 3 {
            inbox.push(format!("{i}"));
        }
        assert_eq!(inbox.take_dropped(), 3);
        assert_eq!(inbox.take_dropped(), 0);
    }
}
