//! The inbox on disk: a writer thread and an atomic replace.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;

type Held = BTreeMap<u64, String>;

/// One sequence number. `None` is a removal.
pub(super) type Change = (u64, Option<String>);

pub(super) fn read(path: &Path) -> Held {
    match std::fs::read(path) {
        Ok(bytes) => rmp_serde::from_slice(&bytes).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "lxmf inbox is unreadable; received messages are lost");
            Held::default()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Held::default(),
        Err(e) => {
            tracing::warn!(error = %e, "lxmf inbox could not be read; received messages are lost");
            Held::default()
        }
    }
}

pub(super) fn spawn_writer(path: PathBuf, mut held: Held) -> Sender<Change> {
    let (tx, rx) = std::sync::mpsc::channel::<Change>();
    thread::spawn(move || {
        while let Ok(change) = rx.recv() {
            apply(&mut held, change);
            while let Ok(next) = rx.try_recv() {
                apply(&mut held, next);
            }
            write_atomically(&path, &held);
        }
    });
    tx
}

/// A sender with no receiver: every send fails, which is the point.
pub(super) fn null_writer() -> Sender<Change> {
    std::sync::mpsc::channel().0
}

fn apply(held: &mut Held, (seq, json): Change) {
    match json {
        Some(json) => held.insert(seq, json),
        None => held.remove(&seq),
    };
}

fn write_atomically(path: &Path, held: &Held) {
    let bytes = match rmp_serde::to_vec(held) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "lxmf inbox could not be encoded");
            return;
        }
    };
    let temp = path.with_extension("tmp");
    if let Err(e) = std::fs::write(&temp, &bytes) {
        tracing::error!(error = %e, "lxmf inbox could not be written");
        return;
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        tracing::error!(error = %e, "lxmf inbox could not be committed");
        let _ = std::fs::remove_file(&temp);
    }
}
