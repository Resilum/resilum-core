use std::collections::HashMap;
use std::path::Path;

use super::registry::Entry;

pub(super) type Kept = HashMap<String, Entry>;

pub(super) fn whatever_the_last_run_left(path: &Path) -> Kept {
    let Ok(bytes) = std::fs::read(path) else {
        return Kept::new();
    };
    let listed: Vec<Entry> = serde_json::from_slice(&bytes).unwrap_or_default();
    listed.into_iter().map(|e| (e.peer.clone(), e)).collect()
}

pub(super) fn write_atomically(path: &Path, held: &Kept) {
    if let Err(e) = write_or_fail(path, held) {
        tracing::warn!(path = %path.display(), error = %e, "the mirror registry could not be written");
    }
}

fn write_or_fail(path: &Path, held: &Kept) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listed: Vec<&Entry> = held.values().collect();
    let tmp = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&tmp)?;
    std::io::Write::write_all(&mut file, &serde_json::to_vec_pretty(&listed)?)?;
    file.sync_all()?;
    std::fs::rename(tmp, path)
}
