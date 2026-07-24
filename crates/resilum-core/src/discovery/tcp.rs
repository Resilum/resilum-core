//! TCP discovery plugin (Tor / I2P / Yggdrasil).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use leviculum_std::api::Node as LevNode;
use leviculum_std::interfaces::TcpClientHandle;
use tokio::sync::Notify;

use super::DiscoveryPlugin;
use super::cache;
use crate::announce_cap::CapController;
use crate::config::{DiscoveryService, EndpointFormat, SocksProxy};

pub struct TcpDiscovered {
    cfg: DiscoveryService,
    engine: Arc<LevNode>,
    // Dropping a TcpClientHandle detaches its interface; hold them here.
    handles: Mutex<HashMap<String, TcpClientHandle>>,
    // Fires on every successful attach so the produce loop re-announces at once.
    trigger: Arc<Notify>,
    // Persistent peer cache; None disables persistence (attach still works).
    cache_path: Option<PathBuf>,
    // Registers each attached interface for adaptive announce-cap control.
    cap_controller: Arc<CapController>,
}

impl TcpDiscovered {
    pub fn new(
        cfg: DiscoveryService,
        engine: Arc<LevNode>,
        trigger: Arc<Notify>,
        cache_path: Option<PathBuf>,
        cap_controller: Arc<CapController>,
    ) -> Self {
        Self {
            cfg,
            engine,
            handles: Mutex::new(HashMap::new()),
            trigger,
            cache_path,
            cap_controller,
        }
    }

    fn detect_host(&self) -> Option<String> {
        if let Some(path) = self.cfg.hostname_path.as_ref() {
            let raw = std::fs::read_to_string(path).ok()?;
            let host = raw.trim();
            return (!host.is_empty()).then(|| host.to_owned());
        }
        if matches!(self.cfg.endpoint_format, EndpointFormat::BracketedIpv6) {
            return crate::net::yggdrasil_local_ipv6().map(|ip| ip.to_string());
        }
        None
    }
}

impl DiscoveryPlugin for TcpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        let host = self.detect_host()?;
        let payload = match self.cfg.endpoint_format {
            EndpointFormat::BracketedIpv6 => format!("[{}]:{}", host, self.cfg.rns_port),
            EndpointFormat::Suffix(_) => format!("{}:{}", host, self.cfg.rns_port),
        };
        Some(payload.into_bytes())
    }

    fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: &[u8]) {
        let Some((host, port)) = parse_endpoint(payload, &self.cfg.endpoint_format) else {
            tracing::debug!(service = %self.cfg.service, "malformed discovery payload");
            return;
        };
        let name = format!("{}[{}]:{}", self.cfg.name_prefix, host, port);
        let mut guard = self.handles.lock().expect("handles");
        if guard.contains_key(&name) {
            return;
        }
        let socks = match &self.cfg.socks_proxy {
            Some(SocksProxy::External(h, p)) => Some((h.clone(), *p)),
            Some(SocksProxy::EmbeddedArti) => {
                tracing::error!(service = %self.cfg.service, "EmbeddedArti was not resolved; skipping peer");
                return;
            }
            None => None,
        };
        match self.engine.spawn_tcp_client(&name, &host, port, socks) {
            Ok(handle) => {
                tracing::info!(service = %self.cfg.service, %name, "attached discovered peer");
                self.cap_controller.attach(handle.id());
                guard.insert(name, handle);
                self.trigger.notify_waiters();
                if let Some(path) = &self.cache_path {
                    let mut records = cache::load(path);
                    cache::upsert(&mut records, payload, cache::now_ts());
                    if let Err(e) = cache::save(path, &records) {
                        tracing::warn!(service = %self.cfg.service, error = %e, "cache save failed");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(service = %self.cfg.service, %name, error = %e, "attach failed");
            }
        }
    }
}

// The char allowlist keeps a malformed announce from injecting weird bytes into
// an interface name (which becomes a log/UI identifier).
fn parse_endpoint(payload: &[u8], format: &EndpointFormat) -> Option<(String, u16)> {
    let s = std::str::from_utf8(payload).ok()?.trim();
    match format {
        EndpointFormat::Suffix(suffix) => {
            let (host, port_str) = s.rsplit_once(':')?;
            let port: u16 = port_str.parse().ok()?;
            if suffix.is_empty() || !host.ends_with(suffix.as_str()) {
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
        EndpointFormat::BracketedIpv6 => {
            let inner = s.strip_prefix('[')?;
            let (host, rest) = inner.split_once(']')?;
            let port: u16 = rest.strip_prefix(':')?.parse().ok()?;
            if !host.bytes().all(|b| b.is_ascii_hexdigit() || b == b':') {
                return None;
            }
            Some((host.to_owned(), port))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn onion() -> EndpointFormat {
        EndpointFormat::Suffix(".onion".into())
    }
    fn i2p() -> EndpointFormat {
        EndpointFormat::Suffix(".b32.i2p".into())
    }

    #[test]
    fn parses_onion_endpoint() {
        let (h, p) = parse_endpoint(b"abc23xyz.onion:4242", &onion()).unwrap();
        assert_eq!(h, "abc23xyz.onion");
        assert_eq!(p, 4242);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let (h, p) = parse_endpoint(b"  peer.b32.i2p:8000\n", &i2p()).unwrap();
        assert_eq!(h, "peer.b32.i2p");
        assert_eq!(p, 8000);
    }

    #[test]
    fn rejects_wrong_suffix() {
        assert!(parse_endpoint(b"peer.b32.i2p:4242", &onion()).is_none());
    }

    #[test]
    fn rejects_injected_chars() {
        assert!(parse_endpoint(b"weird space.onion:4242", &onion()).is_none());
        assert!(parse_endpoint(b"NOTLOWER.onion:4242", &onion()).is_none());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_endpoint(b"peer.onion:70000", &onion()).is_none());
        assert!(parse_endpoint(b"peer.onion:", &onion()).is_none());
        assert!(parse_endpoint(b"peer.onion", &onion()).is_none());
    }

    #[test]
    fn parses_bracketed_ipv6() {
        let (h, p) = parse_endpoint(b"[200:abcd::1]:4242", &EndpointFormat::BracketedIpv6).unwrap();
        assert_eq!(h, "200:abcd::1");
        assert_eq!(p, 4242);
    }

    #[test]
    fn rejects_missing_brackets_or_bad_ipv6_chars() {
        // A tor-style suffix payload must not sneak through the ygg parser.
        assert!(parse_endpoint(b"peer.onion:4242", &EndpointFormat::BracketedIpv6).is_none());
        // Non-hex/colon char inside the brackets is rejected.
        assert!(parse_endpoint(b"[ipv6-here]:4242", &EndpointFormat::BracketedIpv6).is_none());
    }
}
