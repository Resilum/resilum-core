//! Announcing our own address, and dialling the peers who announce theirs.

use std::sync::atomic::Ordering;

use super::TcpDiscovered;
use super::endpoint::{encode_endpoint, parse_endpoint};
use crate::config::SocksProxy;
use crate::discovery::{DiscoveryPlugin, cache};

impl DiscoveryPlugin for TcpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        if !self.active.load(Ordering::Relaxed) {
            return None;
        }
        let host = self.detect_host()?;
        encode_endpoint(&host, self.cfg.rns_port, &self.cfg.endpoint_format)
    }

    fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: Option<&[u8]>) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        let Some((host, port)) = parse_endpoint(payload, &self.cfg.endpoint_format) else {
            tracing::debug!(service = %self.cfg.service, "malformed discovery payload");
            return;
        };
        let name = format!("{}[{}]:{}", self.cfg.name_prefix, host, port);
        // The poisoned lock is recovered from: it is only ever held across a
        // map lookup and an insert, and refusing here would strand the peer.
        let mut guard = self.handles.lock().unwrap_or_else(|e| e.into_inner());
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
                self.origin_registry
                    .record(handle.id(), self.cfg.service.clone());
                guard.insert(name, handle);
                self.trigger.notify_waiters();
                self.remember(payload);
            }
            Err(e) => {
                tracing::warn!(service = %self.cfg.service, %name, error = %e, "attach failed");
            }
        }
    }
}

impl TcpDiscovered {
    /// Keep the peer for the next start, when no announce has arrived yet.
    fn remember(&self, payload: &[u8]) {
        let Some(path) = &self.cache_path else {
            return;
        };
        let mut records = cache::load(path);
        cache::upsert(&mut records, payload, crate::wall_clock::unix_now());
        if let Err(e) = cache::save(path, &records) {
            tracing::warn!(service = %self.cfg.service, error = %e, "cache save failed");
        }
    }
}
