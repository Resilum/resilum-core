//! The queue on disk: a writer thread and an atomic replace.
//!
//! Entries serialise one per line so a line lost to a partial write drops
//! that entry, not the whole queue. Every `Change` here is a decision
//! `Queue` already made — a push, or specific `(event_id, subscriber)`
//! pairs chosen for removal — so the writer only ever applies it; it holds
//! no ceiling, retention or matching logic of its own.

use std::path::{Path, PathBuf};

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};

use super::{Direction, Entry};
use crate::event::decode_hex;
use crate::writer::Writer;

pub(super) type Held = Vec<Entry>;

pub(super) enum Change {
    Push(Entry),
    Remove(Vec<([u8; 32], [u8; 32])>),
}

#[derive(Deserialize)]
struct Line {
    direction: String,
    subscriber: String,
    lxmf: String,
    event_id: String,
    event_json: String,
    queued_at: i64,
}

/// Borrowed so a flush does not copy every held event body, which is what the
/// entries' shared `Arc<str>` exists to avoid.
#[derive(Serialize)]
struct LineRef<'a> {
    direction: &'a str,
    subscriber: String,
    lxmf: String,
    event_id: String,
    event_json: &'a str,
    queued_at: i64,
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
    for line in text.lines().filter(|l| !l.is_empty()) {
        match decode_line(line) {
            Ok(entry) => held.push(entry),
            Err(e) => {
                tracing::warn!(error = %e, "nostr queue line is unreadable; that entry is lost")
            }
        }
    }
    Ok(held)
}

fn decode_line(line: &str) -> Result<Entry, String> {
    let parsed: Line = serde_json::from_str(line).map_err(|e| e.to_string())?;
    Ok(Entry {
        direction: decode_direction(&parsed.direction)?,
        subscriber: decode_hex::<32>(&parsed.subscriber)?,
        lxmf: decode_hex::<16>(&parsed.lxmf)?,
        event_id: decode_hex::<32>(&parsed.event_id)?,
        event_json: parsed.event_json.into(),
        queued_at: parsed.queued_at,
    })
}

fn decode_direction(direction: &str) -> Result<Direction, String> {
    match direction {
        "inbound" => Ok(Direction::Inbound),
        "outbound" => Ok(Direction::Outbound),
        other => Err(format!("unknown direction {other}")),
    }
}

fn encode_direction(direction: Direction) -> &'static str {
    match direction {
        Direction::Inbound => "inbound",
        Direction::Outbound => "outbound",
    }
}

pub(super) fn spawn_writer(path: PathBuf, held: Held) -> Writer<Change> {
    Writer::spawn(path, held, apply, write_atomically)
}

fn apply(held: &mut Held, change: Change) {
    match change {
        Change::Push(entry) => held.push(entry),
        Change::Remove(keys) => held.retain(|e| !keys.contains(&(e.event_id, e.subscriber))),
    }
}

pub(super) fn null_writer() -> Writer<Change> {
    Writer::null()
}

fn write_atomically(path: &Path, held: &Held) {
    let mut text = String::new();
    for entry in held {
        let line = LineRef {
            direction: encode_direction(entry.direction),
            subscriber: HEXLOWER.encode(&entry.subscriber),
            lxmf: HEXLOWER.encode(&entry.lxmf),
            event_id: HEXLOWER.encode(&entry.event_id),
            event_json: &entry.event_json,
            queued_at: entry.queued_at,
        };
        match serde_json::to_string(&line) {
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
