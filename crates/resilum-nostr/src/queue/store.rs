//! The queue on disk: a writer thread and an atomic replace.
//!
//! Entries serialise one per line so a line lost to a partial write drops
//! that entry, not the whole queue. Every `Change` here is a decision
//! `Queue` already made, so the writer only ever applies it; it holds no
//! ceiling, retention or matching logic of its own.

use std::path::{Path, PathBuf};

use super::{Entry, Handoff};
use resilum_core::storage::Writer;

mod line;

pub(super) type Held = Vec<Entry>;

pub(super) enum Change {
    Push(Entry),
    Remove(Vec<([u8; 32], [u8; 32])>),
    Handoff {
        key: ([u8; 32], [u8; 32]),
        handoff: Handoff,
    },
}

/// A file we cannot read is not an empty one. Carrying on with an empty map
/// would have the first mutation rewrite the file from it, so a transient
/// fault would destroy every queued event; refusing to open says so instead.
pub(super) fn read(path: &Path) -> Result<Held, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Held::new()),
        Err(e) => {
            return Err(format!(
                "the nostr queue at {} is unreadable: {e}",
                path.display()
            ));
        }
    };
    let mut held = Held::new();
    for text in text.lines().filter(|l| !l.is_empty()) {
        match line::decode(text) {
            Ok(entry) => held.push(entry),
            Err(e) => {
                tracing::warn!(error = %e, "nostr queue line is unreadable; that entry is lost")
            }
        }
    }
    Ok(held)
}

pub(super) fn spawn_writer(path: PathBuf, held: Held) -> Writer<Change> {
    Writer::spawn(path, held, apply, write_atomically)
}

fn apply(held: &mut Held, change: Change) {
    match change {
        Change::Push(entry) => held.push(entry),
        Change::Remove(keys) => held.retain(|e| !keys.contains(&(e.event_id, e.subscriber))),
        Change::Handoff { key, handoff } => {
            if let Some(entry) = held.iter_mut().find(|e| (e.event_id, e.subscriber) == key) {
                entry.handoff = handoff;
            }
        }
    }
}

pub(super) fn null_writer() -> Writer<Change> {
    Writer::nowhere_to_write()
}

fn write_atomically(path: &Path, held: &Held) {
    let mut text = String::new();
    for entry in held {
        match line::encode(entry) {
            Ok(json) => {
                text.push_str(&json);
                text.push('\n');
            }
            Err(e) => tracing::error!(error = %e, "nostr queue entry could not be encoded"),
        }
    }
    let temp = path.with_extension("tmp");
    if let Err(e) = std::fs::write(&temp, &text) {
        tracing::error!(error = %e, "nostr queue could not be written");
        return;
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        tracing::error!(error = %e, "nostr queue could not be committed");
        let _ = std::fs::remove_file(&temp);
    }
}
