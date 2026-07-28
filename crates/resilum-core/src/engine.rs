//! Renders the Reticulum INI config from `Config` and builds a leviculum node.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use leviculum_std::api::{Identity, NodeBuilder};

use crate::{Config, Error, Result, identity};

pub(crate) fn render_config(config: &Config) -> String {
    let discover = config.discover_interfaces;
    let mut out = String::new();
    let _ = writeln!(out, "[reticulum]");
    let _ = writeln!(out, "  enable_transport = yes");
    let _ = writeln!(out, "  share_instance = yes");
    let _ = writeln!(out, "  instance_name = {}", config.instance_name);
    let _ = writeln!(out, "  discover_interfaces = {}", yes_no(discover));
    let cap = if discover { config.autoconnect_max } else { 0 };
    let _ = writeln!(out, "  autoconnect_discovered_interfaces = {cap}");
    if let Some(identity) = &config.network_identity {
        let _ = writeln!(out, "  network_identity = {}", identity.display());
    }
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
        if let Some(addr) = &config.reachable_on {
            let _ = writeln!(out, "    reachable_on = {addr}");
        }
        if let Some(name) = &config.discovery_name {
            let _ = writeln!(out, "    discovery_name = {name}");
        }
    }
    for (i, anchor) in config.bootstrap.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        let _ = write!(
            out,
            "\n  [[Bootstrap {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n"
        );
    }
    for (i, anchor) in config.bootstrap_only.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        let _ = write!(
            out,
            "\n  [[Bootstrap-only {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n    bootstrap_only = yes\n"
        );
    }
    if let Some(i2p) = &config.i2p {
        let _ = write!(
            out,
            "\n  [[I2P]]\n    type = I2PInterface\n    enabled = yes\n"
        );
        if i2p.connectable {
            let _ = writeln!(out, "    connectable = yes");
        }
        if !i2p.peers.is_empty() {
            let _ = writeln!(out, "    peers = {}", i2p.peers.join(", "));
        }
    }
    for covert in &config.specs.covert {
        let Some(command) = &covert.command else {
            continue;
        };
        let _ = write!(
            out,
            "\n  [[covert/{}]]\n    type = PipeInterface\n    enabled = yes\n    command = {command}\n",
            covert.carrier
        );
    }
    out
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// Split `host:port` into `(host, port)`. Brackets are kept: leviculum rebuilds
/// the address as `format!("{host}:{port}")`, which needs `[..]` for IPv6.
fn split_host_port(value: &str) -> (&str, &str) {
    match value.rsplit_once(':') {
        Some((host, port)) => (host, port),
        None => (value, ""),
    }
}

/// Returns the identity too, so egress destinations bind to the same one.
pub(crate) fn build_node(
    config: &Config,
    protect: Option<leviculum_std::socket_hook::OutboundSocketHook>,
) -> Result<(NodeBuilder, Identity)> {
    let dir = config
        .storage_path
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("resilum-{}", config.instance_name)));
    fs::create_dir_all(&dir).map_err(|e| Error::Config(format!("config dir: {e}")))?;
    let config_path: PathBuf = dir.join("config");
    fs::write(&config_path, render_config(config))
        .map_err(|e| Error::Config(format!("write config: {e}")))?;
    let identity = identity::load_or_create(&dir);
    // Pre-create at 0600; leviculum would otherwise write it world-readable.
    if let Some(network_identity) = &config.network_identity {
        identity::load_or_create_at(&resolve_under(network_identity, &dir));
    }
    let mut builder = NodeBuilder::new()
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
        assert!(ini.contains("listen_ip = [::]"));
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

    #[test]
    fn default_network_renders_anchors_and_discovery() {
        let ini = render_config(&Config::default_network("node"));
        assert!(ini.contains("listen_port = 4242"));
        assert!(ini.contains("discovery_name = resilum"));
        assert!(ini.contains("network_identity = network_identity"));
        assert!(ini.contains("target_host = istanbul.reserve.network"));
        assert!(ini.contains("bootstrap_only = yes"));
        assert!(ini.contains("target_host = [200:3953:999b:282e:e526:bcd2:c329:31a]"));
    }

    #[test]
    fn renders_i2p_interface() {
        let cfg = Config {
            i2p: Some(crate::config::I2pInterface {
                connectable: true,
                peers: vec!["a.b32.i2p".into()],
            }),
            ..Config::minimal("test")
        };
        let ini = render_config(&cfg);
        assert!(ini.contains("type = I2PInterface"));
        assert!(ini.contains("connectable = yes"));
        assert!(ini.contains("peers = a.b32.i2p"));
    }

    #[test]
    fn covert_spec_renders_a_pipe_interface() {
        let mut cfg = Config::minimal("test");
        cfg.specs =
            crate::spec::load("covert:\n  - carrier: icmp\n    command: rns-over-icmp --peer x\n")
                .unwrap();
        let ini = render_config(&cfg);
        assert!(ini.contains("[[covert/icmp]]"));
        assert!(ini.contains("type = PipeInterface"));
        assert!(ini.contains("command = rns-over-icmp --peer x"));
    }
}
