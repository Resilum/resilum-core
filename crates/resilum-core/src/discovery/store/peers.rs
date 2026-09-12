use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use super::records::{Record, prune, top_n, upsert};
use super::{TOP_N_ACTIVE, TTL_SECONDS};
use crate::storage::Writer;

type Records = Vec<Record>;

pub(crate) struct Peers {
    remembered: Mutex<Records>,
    writer: Writer<Records>,
}

impl Peers {
    pub(crate) fn open(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self {
                remembered: Mutex::new(Vec::new()),
                writer: Writer::nowhere_to_write(),
            };
        };
        let remembered = whatever_the_last_run_left(&path);
        Self {
            writer: Writer::spawn(path, remembered.clone(), replace, write_atomically),
            remembered: Mutex::new(remembered),
        }
    }

    pub(crate) fn seen(&self, endpoint: &[u8], now: f64) {
        let mut remembered = self.lock();
        upsert(&mut remembered, endpoint, now);
        self.writer.send(remembered.clone());
    }

    pub(crate) fn forget_stale(&self, now: f64) {
        let mut remembered = self.lock();
        if prune(&mut remembered, TTL_SECONDS, now) == 0 {
            return;
        }
        self.writer.send(remembered.clone());
    }

    pub(crate) fn most_recent(&self) -> Vec<Vec<u8>> {
        top_n(&self.lock(), TOP_N_ACTIVE)
    }

    fn lock(&self) -> MutexGuard<'_, Records> {
        self.remembered.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn replace(held: &mut Records, latest: Records) {
    *held = latest;
}

fn whatever_the_last_run_left(path: &Path) -> Records {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_atomically(path: &Path, held: &Records) {
    if let Err(e) = write_or_fail(path, held) {
        tracing::warn!(path = %path.display(), error = %e, "the peer cache could not be written");
    }
}

fn write_or_fail(path: &Path, held: &[Record]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&tmp)?;
    std::io::Write::write_all(&mut file, &serde_json::to_vec_pretty(held)?)?;
    file.sync_all()?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
#[path = "peers_tests.rs"]
mod tests;
