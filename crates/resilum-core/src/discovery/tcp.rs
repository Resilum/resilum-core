//! Suffix-based TCP discovery plugin (Tor/I2P).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use leviculum_std::api::Node as LevNode;
use leviculum_std::interfaces::TcpClientHandle;

use super::DiscoveryPlugin;
use crate::config::DiscoveryService;

pub struct TcpDiscovered {
    cfg: DiscoveryService,
    engine: Arc<LevNode>,
    // Dropping a TcpClientHandle detaches its interface; hold them here.
    handles: Mutex<HashMap<String, TcpClientHandle>>,
}

impl TcpDiscovered {
    pub fn new(cfg: DiscoveryService, engine: Arc<LevNode>) -> Self {
        Self {
            cfg,
            engine,
            handles: Mutex::new(HashMap::new()),
        }
    }
}

impl DiscoveryPlugin for TcpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        let host = std::fs::read_to_string(self.cfg.hostname_path.as_ref()?).ok()?;
        let host = host.trim();
        if host.is_empty() {
            return None;
        }
        Some(format!("{}:{}", host, self.cfg.rns_port).into_bytes())
    }

    fn consume_endpoint(&self, payload: &[u8]) {
        let Some((host, port)) = parse_endpoint(payload, &self.cfg.host_suffix) else {
            tracing::debug!(service = %self.cfg.service, "malformed discovery payload");
            return;
        };
        let name = format!("{}[{}]:{}", self.cfg.name_prefix, host, port);
        let mut guard = self.handles.lock().expect("handles");
        if guard.contains_key(&name) {
            return;
        }
        match self
            .engine
            .spawn_tcp_client(&name, &host, port, self.cfg.socks_proxy.clone())
        {
            Ok(handle) => {
                tracing::info!(service = %self.cfg.service, %name, "attached discovered peer");
                guard.insert(name, handle);
            }
            Err(e) => {
                tracing::warn!(service = %self.cfg.service, %name, error = %e, "attach failed");
            }
        }
    }
}

// The char allowlist keeps a malformed announce from injecting weird bytes into
// an interface name (which becomes a log/UI identifier).
fn parse_endpoint(payload: &[u8], suffix: &str) -> Option<(String, u16)> {
    let s = std::str::from_utf8(payload).ok()?.trim();
    let (host, port_str) = s.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    if suffix.is_empty() || !host.ends_with(suffix) {
        return None;
    }
    if !host
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
    {
        return None;
    }
    Some((host.to_owned(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_onion_endpoint() {
        let (h, p) = parse_endpoint(b"abc23xyz.onion:4242", ".onion").unwrap();
        assert_eq!(h, "abc23xyz.onion");
        assert_eq!(p, 4242);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let (h, p) = parse_endpoint(b"  peer.b32.i2p:8000\n", ".b32.i2p").unwrap();
        assert_eq!(h, "peer.b32.i2p");
        assert_eq!(p, 8000);
    }

    #[test]
    fn rejects_wrong_suffix() {
        assert!(parse_endpoint(b"peer.b32.i2p:4242", ".onion").is_none());
    }

    #[test]
    fn rejects_injected_chars() {
        assert!(parse_endpoint(b"weird space.onion:4242", ".onion").is_none());
        assert!(parse_endpoint(b"NOTLOWER.onion:4242", ".onion").is_none());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_endpoint(b"peer.onion:70000", ".onion").is_none());
        assert!(parse_endpoint(b"peer.onion:", ".onion").is_none());
        assert!(parse_endpoint(b"peer.onion", ".onion").is_none());
    }
}
