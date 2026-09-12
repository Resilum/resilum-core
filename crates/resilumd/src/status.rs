//! The snapshot the node leaves behind for anyone asking from outside it.

mod show;

use std::path::{Path, PathBuf};

use resilum_core::Node;

pub use show::run;

pub fn file_path(storage_path: Option<&Path>, env: Option<String>) -> PathBuf {
    if let Some(p) = env.filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    match storage_path {
        Some(dir) => dir.join("status.json"),
        None => std::env::temp_dir().join("resilum-status.json"),
    }
}

pub fn leave_behind(node: &Node, path: &Path) {
    let snapshot = resilum_core::status::snapshot(node);
    let json = match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => json,
        Err(error) => {
            tracing::warn!(%error, "status is not representable as JSON");
            return;
        }
    };
    if let Err(error) = resilum_store::write_text(path, &json) {
        tracing::warn!(path = %path.display(), %error, "status write failed");
    }
}
