//! Load the daemon's YAML config file into a `resilum_core::Config`.

use std::path::{Path, PathBuf};

use resilum_core::Config;

pub fn load(path: &Path) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut cfg = resilum_core::from_yaml(&raw)?;
    if cfg.storage_path.is_none() {
        cfg.storage_path = Some(beside(path));
    }
    Ok(cfg)
}

fn beside(config: &Path) -> PathBuf {
    config
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .join("state")
}

#[cfg(test)]
mod tests;
