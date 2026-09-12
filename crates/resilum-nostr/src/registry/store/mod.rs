//! The registry on disk: a writer thread and an atomic replace.
//!
//! Records serialise one per line so a line lost to a partial write drops
//! that subscriber, not every subscriber on file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::batch::BatchId;
use resilum_store::Writer;

mod line;
mod migrate;

pub(super) type Held = BTreeMap<[u8; 32], Record>;

pub(super) enum Change {
    Upsert([u8; 32], Record),
    Remove(Vec<[u8; 32]>),
}

#[derive(Clone, Copy)]
pub(super) struct Record {
    pub(super) lxmf: [u8; 16],
    pub(super) created_at: i64,
    pub(super) last_seen: i64,
    pub(super) batch: BatchId,
}

/// A line written before batching existed has no `batch`; `migrate::assign`
/// gives it one and, since that changes what is true on disk, `read`
/// rewrites the file so a second restart does not have to redo the work.
///
/// A file we cannot read is not an empty one: carrying on empty would have
/// the first mutation rewrite it and drop every subscriber.
pub(super) fn read(path: &Path) -> Result<Held, String> {
    let text = match resilum_store::read_text(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Held::default()),
        Err(e) => {
            return Err(format!(
                "the nostr registry at {} is unreadable: {e}",
                path.display()
            ));
        }
    };
    let mut loaded = Vec::new();
    for entry in text.lines().filter(|l| !l.is_empty()) {
        match line::decode(entry) {
            Ok(entry) => loaded.push(entry),
            Err(e) => {
                tracing::warn!(error = %e, "nostr registry line is unreadable; that subscriber is lost")
            }
        }
    }
    let migration = migrate::assign(loaded);
    if migration.rewrite_needed {
        write_atomically(path, &migration.held);
    }
    Ok(migration.held)
}

pub(super) fn spawn_writer(path: PathBuf, held: Held) -> Writer<Change> {
    Writer::spawn(path, held, apply, write_atomically)
}

fn apply(held: &mut Held, change: Change) {
    match change {
        Change::Upsert(pubkey, record) => {
            held.insert(pubkey, record);
        }
        Change::Remove(pubkeys) => held.retain(|pubkey, _| !pubkeys.contains(pubkey)),
    }
}

pub(super) fn null_writer() -> Writer<Change> {
    Writer::nowhere_to_write()
}

/// Atomic against a crash and not only against a torn read: the replacement
/// is flushed to the platter before the rename, and the directory after it,
/// so a machine that loses power mid-write comes back to one whole file or
/// the other and never to a rename the kernel had not yet recorded.
fn write_atomically(path: &Path, held: &Held) {
    let mut text = String::new();
    for (pubkey, record) in held {
        match serde_json::to_string(&line::encode(pubkey, record)) {
            Ok(json) => {
                text.push_str(&json);
                text.push('\n');
            }
            Err(e) => tracing::error!(error = %e, "nostr registry entry could not be encoded"),
        }
    }
    if let Err(e) = resilum_store::replace_with(path, text.as_bytes()) {
        tracing::error!(error = %e, "nostr registry could not be written");
    }
}
