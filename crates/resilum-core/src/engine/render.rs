//! Renders the Reticulum INI config from `Config`.

use crate::Config;
use crate::text::Text as _;

pub(crate) fn render_config(config: &Config, data_port: u16) -> String {
    let discover = config.discover_interfaces;
    let mut out = String::new();
    out.line("[reticulum]");
    out.line("  enable_transport = yes");
    out.line("  share_instance = yes");
    out.line(format_args!("  instance_name = {}", config.instance_name));
    out.line(format_args!("  discover_interfaces = {}", yes_no(discover)));
    let cap = if discover { config.autoconnect_max } else { 0 };
    out.line(format_args!("  autoconnect_discovered_interfaces = {cap}"));
    out.line("  control_channel_capacity = 16384");
    if let Some(identity) = &config.network_identity {
        out.line(format_args!("  network_identity = {}", identity.display()));
    }
    out.line("\n[interfaces]");

    if discover {
        out.block(format_args!(
            "\n  [[LAN AutoDiscovery]]\n    type = AutoInterface\n    enabled = yes\n    \
             data_port = {data_port}\n"
        ));
    }
    if let Some(listen) = &config.listen {
        let (host, port) = split_host_port(listen);
        out.block(format_args!(
            "\n  [[Public TCP listener]]\n    type = TCPServerInterface\n    enabled = yes\n    \
             listen_ip = {host}\n    listen_port = {port}\n    discoverable = yes\n    \
             mode = gateway\n"
        ));
        if let Some(addr) = &config.reachable_on {
            out.line(format_args!("    reachable_on = {addr}"));
        }
        if let Some(name) = &config.discovery_name {
            out.line(format_args!("    discovery_name = {name}"));
        }
    }
    for (i, anchor) in config.bootstrap.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        out.block(format_args!(
            "\n  [[Bootstrap {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n    mode = boundary\n"
        ));
    }
    for (i, anchor) in config.bootstrap_only.iter().enumerate() {
        let (host, port) = split_host_port(anchor);
        out.block(format_args!(
            "\n  [[Bootstrap-only {i}]]\n    type = TCPClientInterface\n    enabled = yes\n    \
             target_host = {host}\n    target_port = {port}\n    bootstrap_only = yes\n    \
             mode = boundary\n"
        ));
    }
    if let Some(udp) = config.udp.as_ref().filter(|udp| !udp.carries_nothing()) {
        let (host, port) = split_host_port(udp.bound_to());
        out.block(format_args!(
            "\n  [[UDP]]\n    type = UDPInterface\n    enabled = yes\n    listen_ip = {host}\n    \
             listen_port = {port}\n    forward_ip = {}\n",
            udp.peers_every_datagram_goes_to.join(", ")
        ));
    }
    if let Some(i2p) = &config.i2p {
        out.block("\n  [[I2P]]\n    type = I2PInterface\n    enabled = yes\n    mode = boundary\n");
        if i2p.connectable {
            out.line("    connectable = yes");
        }
        if !i2p.peers.is_empty() {
            out.line(format_args!("    peers = {}", i2p.peers.join(", ")));
        }
    }
    for covert in &config.specs.covert {
        let Some(command) = &covert.command else {
            continue;
        };
        out.block(format_args!(
            "\n  [[covert/{}]]\n    type = PipeInterface\n    enabled = yes\n    command = {command}\n",
            covert.carrier
        ));
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
mod tests;
