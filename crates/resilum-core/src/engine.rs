//! Builds a leviculum node from the rendered Reticulum config.

use std::path::PathBuf;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNodeBuilder;

use crate::{Config, Error, Result, identity};

mod a_data_port;
mod names;
mod render;
use render::render_config;

/// Returns the identity too, so egress destinations bind to the same one.
pub(crate) fn build_node(
    config: &Config,
    protect: Option<leviculum_std::socket_hook::OutboundSocketHook>,
) -> Result<(ReticulumNodeBuilder, Identity)> {
    let dir = config
        .storage_path
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("resilum-{}", config.instance_name)));
    resilum_store::make_room_for(&dir).map_err(|e| Error::Config(format!("config dir: {e}")))?;
    let config_path: PathBuf = dir.join("config");
    let data_port = a_data_port::nobody_else_holds(a_data_port::WHAT_RETICULUM_EXPECTS);
    let reachable = names::only_those_that_resolve(config);
    resilum_store::write_text(&config_path, &render_config(&reachable, data_port))
        .map_err(|e| Error::Config(format!("write config: {e}")))?;
    let identity = match &config.identity_private_base64 {
        Some(b64) => identity::from_base64(b64)
            .ok_or_else(|| Error::Config("invalid identity_private_base64".into()))?,
        None => identity::load_or_create(&dir),
    };
    // Pre-create at 0600; leviculum would otherwise write it world-readable.
    if let Some(network_identity) = &config.network_identity {
        identity::load_or_create_at(&resolve_under(network_identity, &dir));
    }
    let mut builder = ReticulumNodeBuilder::new()
        .identity(identity.clone())
        .storage_path(dir)
        .config_file(config_path);
    if let Some(hook) = protect {
        builder = builder.outbound_socket_hook(hook);
    }
    Ok((builder, identity))
}

/// Mirrors leviculum's path resolution: `~/` expands, relative resolves under `storage`.
fn resolve_under(path: &std::path::Path, storage: &std::path::Path) -> PathBuf {
    let expanded = match path.strip_prefix("~") {
        Ok(rest) => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => path.to_path_buf(),
        },
        Err(_) => path.to_path_buf(),
    };
    if expanded.is_absolute() {
        expanded
    } else {
        storage.join(expanded)
    }
}
