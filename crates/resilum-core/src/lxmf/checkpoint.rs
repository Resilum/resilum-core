//! Durable router state. `LxmfRouter::persist` runs under the core lock, so
//! this storage hands each change to a writer thread instead of writing it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;

use leviculum_lxmf::storage::{LxmfStorage, StorageError};

type Slots = BTreeMap<Vec<u8>, Vec<u8>>;

/// One changed key. `None` is a removal.
type Change = (Vec<u8>, Option<Vec<u8>>);

pub(super) struct Checkpoint {
    slots: Slots,
    writer: Sender<Change>,
    writer_lost: bool,
}

impl Checkpoint {
    /// A missing checkpoint is nothing to restore; one that exists and will not
    /// read means the queue silently starts empty, so that is reported.
    pub(super) fn open(path: PathBuf) -> Self {
        let slots = match std::fs::read(&path) {
            Ok(bytes) => rmp_serde::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "lxmf checkpoint is unreadable; starting with an empty queue");
                Slots::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Slots::default(),
            Err(e) => {
                tracing::warn!(error = %e, "lxmf checkpoint could not be read; starting with an empty queue");
                Slots::default()
            }
        };
        Self {
            writer: spawn_writer(path, slots.clone()),
            slots,
            writer_lost: false,
        }
    }

    /// Reported once: the router checkpoints on every queue change, so logging
    /// each failure would bury the first.
    fn record(&mut self, key: &[u8], value: Option<Vec<u8>>) {
        if self.writer.send((key.to_vec(), value)).is_err() && !self.writer_lost {
            self.writer_lost = true;
            tracing::error!("lxmf checkpoint writer is gone; the queue is no longer durable");
        }
    }
}

impl LxmfStorage for Checkpoint {
    fn load(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.slots.get(key).cloned())
    }

    fn store(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.slots.insert(key.to_vec(), value.to_vec());
        self.record(key, Some(value.to_vec()));
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.slots.remove(key);
        self.record(key, None);
        Ok(())
    }

    fn keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, StorageError> {
        Ok(self
            .slots
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect())
    }
}

/// Coalesces changes that arrived during the previous write: the router asks
/// far more often than a disk wants.
fn spawn_writer(path: PathBuf, mut slots: Slots) -> Sender<Change> {
    let (tx, rx) = std::sync::mpsc::channel::<Change>();
    thread::spawn(move || {
        while let Ok(change) = rx.recv() {
            apply(&mut slots, change);
            while let Ok(next) = rx.try_recv() {
                apply(&mut slots, next);
            }
            write_atomically(&path, &slots);
        }
    });
    tx
}

fn apply(slots: &mut Slots, (key, value): Change) {
    match value {
        Some(value) => slots.insert(key, value),
        None => slots.remove(&key),
    };
}

/// Temporary plus rename, so a kill mid-write leaves the previous checkpoint
/// intact. Failures are reported: the node otherwise goes on believing its
/// queue is durable.
fn write_atomically(path: &Path, slots: &Slots) {
    let bytes = match rmp_serde::to_vec(slots) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "lxmf checkpoint could not be encoded");
            return;
        }
    };
    let temp = path.with_extension("tmp");
    if let Err(e) = std::fs::write(&temp, &bytes) {
        tracing::error!(error = %e, "lxmf checkpoint could not be written");
        return;
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        tracing::error!(error = %e, "lxmf checkpoint could not be committed");
        let _ = std::fs::remove_file(&temp);
    }
}
