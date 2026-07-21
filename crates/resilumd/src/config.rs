//! Load the daemon's YAML config file into a `resilum_core::Config`.

use std::path::Path;

use resilum_core::Config;

pub fn load(path: &Path) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    resilum_core::from_yaml(&raw)
}
