//! The writer thread both stores keep: changes cross a channel, and dropping
//! the store waits for everything it sent to reach the disk.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

pub(crate) struct Writer<C> {
    changes: Option<Sender<C>>,
    thread: Option<JoinHandle<()>>,
}

impl<C: Send + 'static> Writer<C> {
    /// Applies each change to `held` and rewrites the file once the burst is
    /// drained, so a run of changes costs one write rather than one apiece.
    pub(crate) fn spawn<H: Send + 'static>(
        path: PathBuf,
        mut held: H,
        apply: fn(&mut H, C),
        write: fn(&Path, &H),
    ) -> Self {
        let (changes, rx) = std::sync::mpsc::channel::<C>();
        let thread = std::thread::spawn(move || {
            while let Ok(change) = rx.recv() {
                apply(&mut held, change);
                while let Ok(change) = rx.try_recv() {
                    apply(&mut held, change);
                }
                write(&path, &held);
            }
        });
        Self {
            changes: Some(changes),
            thread: Some(thread),
        }
    }

    /// For a store with nowhere to write: a change is taken and dropped.
    pub(crate) fn null() -> Self {
        Self {
            changes: None,
            thread: None,
        }
    }

    pub(crate) fn send(&self, change: C) {
        let Some(changes) = &self.changes else { return };
        // A store with somewhere to write has a live thread for the whole of
        // its life, so this can only mean the thread has died: in-memory
        // state carries on being changed while the disk stops following it.
        if changes.send(change).is_err() {
            tracing::error!("the writer thread is gone; changes no longer reach the disk");
        }
    }
}

/// Closing the channel is what ends the thread, so the sender goes first;
/// joining while still holding it would wait forever. Dropping the store is
/// therefore the only point at which every change it sent is certainly on
/// disk.
impl<C> Drop for Writer<C> {
    fn drop(&mut self) {
        self.changes = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
