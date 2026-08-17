use std::time::Duration;

use data_encoding::HEXLOWER;
use leviculum_std::DestinationHash;
use serde::Deserialize;

use crate::config::LxmfConfig;

/// `lxmf:` — a top-level section rather than a discovery entry, because LXMF is
/// not a transport: it addresses the node's own identity and rides whatever
/// interfaces are already up.
#[derive(Deserialize)]
pub(in crate::config_file) struct LxmfFile {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    announce_interval_secs: Option<u64>,
    /// 32-hex propagation node to pin. Omit to use the nearest one announced.
    #[serde(default)]
    propagation_node: Option<String>,
}

impl From<LxmfFile> for LxmfConfig {
    fn from(f: LxmfFile) -> Self {
        let mut cfg = LxmfConfig {
            display_name: f.display_name,
            propagation_node: f.propagation_node.as_deref().and_then(parse_destination),
            ..LxmfConfig::default()
        };
        if let Some(secs) = f.announce_interval_secs {
            cfg.announce_interval = Duration::from_secs(secs);
        }
        cfg
    }
}

/// A malformed hash falls back to automatic selection rather than failing the
/// whole config: messaging still works, it just picks its own node.
fn parse_destination(hex: &str) -> Option<DestinationHash> {
    let bytes = HEXLOWER.decode(hex.as_bytes()).ok()?;
    match <[u8; 16]>::try_from(bytes.as_slice()) {
        Ok(bytes) => Some(DestinationHash::new(bytes)),
        Err(_) => {
            tracing::warn!(%hex, "lxmf propagation_node is not a 16-byte destination, ignoring");
            None
        }
    }
}
