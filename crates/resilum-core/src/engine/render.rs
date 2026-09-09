//! Renders the Reticulum INI config from `Config`.

use std::fmt::Write as _;

use crate::Config;

pub(crate) fn render_config(config: &Config, data_port: u16) -> String {
    let discover = config.discover_interfaces;
    let mut out = String::new();
    let _ = writeln!(out, "[reticulum]");
    let _ = writeln!(out, "  enable_transport = yes");
    let _ = writeln!(out, "  share_instance = yes");
    let _ = writeln!(out, "  instance_name = {}", config.instance_name);
    let _ = writeln!(out, "  discover_interfaces = {}", yes_no(discover));
    let cap = if discover { config.autoconnect_max } else { 0 };
    let _ = writeln!(out, "  autoconnect_discovered_interfaces = {cap}");
    let _ = writeln!(out, "  control_channel_capacity = 16384");
    if let Some(identity) = &config.network_identity {
        let _ = writeln!(out, "  network_identity = {}", identity.display());
    }
    let _ = writeln!(out, "\n[interfaces]");

    if discover {
        let _ = write!(
            out,
            "\n  [[LAN AutoDiscovery]]\n    type = AutoInterface\n    enabled = yes\n    \
             data_port = {data_port}\n"
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
             target_host = {host}\n    target_port = {port}\n    mode = boundary\n"
        );
    }
    for (i, anchor) in config.bootstrap_only.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        let _ = write!(
            out,
            "\n  [[Bootstrap-only {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n    bootstrap_only = yes\n    \
             mode = boundary\n"
        );
    }
    if let Some(udp) = config.udp.as_ref().filter(|udp| !udp.carries_nothing()) {
        let (host, port) = split_host_port(udp.bound_to());
        let _ = write!(
            out,
            "\n  [[UDP]]\n    type = UDPInterface\n    enabled = yes\n    listen_ip = {host}\n    \
             listen_port = {port}\n    forward_ip = {}\n",
            udp.peers_every_datagram_goes_to.join(", ")
        );
    }
    if let Some(i2p) = &config.i2p {
        let _ = write!(
            out,
            "\n  [[I2P]]\n    type = I2PInterface\n    enabled = yes\n    mode = boundary\n"
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

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
