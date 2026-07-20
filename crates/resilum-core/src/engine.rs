//! Renders a Reticulum config file from `Config` and builds a leviculum node
//! from it. Interfaces, discovery auto-connect and (later) Pipe transports all
//! come from the rendered config, driven the same way as the project.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use leviculum_std::api::{self, NodeBuilder};

use crate::{Config, Error, Result};

/// Auto-connect cap for discovered interfaces (parity).
const AUTOCONNECT_MAX: usize = 5;

/// Render the-compatible Reticulum INI config from `Config`.
pub(crate) fn render_config(config: &Config) -> String {
    let discover = config.discover_interfaces;
    let mut out = String::new();
    let _ = writeln!(out, "[reticulum]");
    let _ = writeln!(out, "  enable_transport = yes");
    let _ = writeln!(out, "  share_instance = yes");
    let _ = writeln!(out, "  instance_name = {}", config.instance_name);
    let _ = writeln!(out, "  discover_interfaces = {}", yes_no(discover));
    let cap = if discover { AUTOCONNECT_MAX } else { 0 };
    let _ = writeln!(out, "  autoconnect_discovered_interfaces = {cap}");
    let _ = writeln!(out, "\n[interfaces]");

    if discover {
        let _ = write!(
            out,
            "\n  [[LAN AutoDiscovery]]\n    type = AutoInterface\n    enabled = yes\n"
        );
    }
    if let Some(listen) = &config.listen {
        let (host, port) = split_host_port(listen);
        let _ = write!(
            out,
            "\n  [[Public TCP listener]]\n    type = TCPServerInterface\n    enabled = yes\n    \
             listen_ip = {host}\n    listen_port = {port}\n    discoverable = yes\n    \
             mode = gateway\n"
        );
    }
    for (i, anchor) in config.bootstrap.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        let _ = write!(
            out,
            "\n  [[Bootstrap {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n"
        );
    }
    out
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// Split `host:port` (IPv6 in brackets) into `(host, port)`.
fn split_host_port(value: &str) -> (&str, &str) {
    match value.rsplit_once(':') {
        Some((host, port)) => (host.trim_start_matches('[').trim_end_matches(']'), port),
        None => (value, ""),
    }
}

/// Write the rendered config to disk and build a leviculum node from it.
pub(crate) fn build_node(config: &Config) -> Result<NodeBuilder> {
    let dir = config
        .storage_path
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("resilum-{}", config.instance_name)));
    fs::create_dir_all(&dir).map_err(|e| Error::Config(format!("config dir: {e}")))?;
    let config_path: PathBuf = dir.join("config");
    fs::write(&config_path, render_config(config))
        .map_err(|e| Error::Config(format!("write config: {e}")))?;
    // TODO: load-or-generate a stable identity from `storage_path`.
    Ok(NodeBuilder::new()
        .identity(api::generate_identity())
        .storage_path(dir)
        .config_file(config_path))
}

#[cfg(test)]
mod tests {
    use super::render_config;
    use crate::Config;

    #[test]
    fn renders_listener_bootstrap_and_autoconnect() {
        let cfg = Config {
            listen: Some("[::]:4242".into()),
            bootstrap: vec!["anchor.example:4343".into()],
            ..Config::minimal("test")
        };
        let ini = render_config(&cfg);
        assert!(ini.contains("autoconnect_discovered_interfaces = 5"));
        assert!(ini.contains("type = AutoInterface"));
        assert!(ini.contains("listen_ip = ::"));
        assert!(ini.contains("listen_port = 4242"));
        assert!(ini.contains("target_host = anchor.example"));
        assert!(ini.contains("target_port = 4343"));
    }

    #[test]
    fn no_discovery_disables_autoconnect_and_auto_interface() {
        let cfg = Config {
            discover_interfaces: false,
            ..Config::minimal("x")
        };
        let ini = render_config(&cfg);
        assert!(ini.contains("discover_interfaces = no"));
        assert!(ini.contains("autoconnect_discovered_interfaces = 0"));
        assert!(!ini.contains("AutoInterface"));
    }
}
