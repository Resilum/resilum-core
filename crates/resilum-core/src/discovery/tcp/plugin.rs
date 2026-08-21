//! Announcing our own address, and dialling the peers who announce theirs.

use std::sync::atomic::Ordering;
use std::time::Duration;

use leviculum_std::api::Identity;

use super::endpoint::{encode_endpoint, parse_endpoint};
use super::{Attached, TcpDiscovered, quota};
use crate::config::SocksProxy;
use crate::coordinates::PeerId;
use crate::discovery::{DiscoveryPlugin, cache};

fn who_announced(announcer_pubkey: Option<&[u8]>) -> Option<PeerId> {
    Identity::from_public_key_bytes(announcer_pubkey?)
        .ok()
        .map(|identity| *identity.hash())
}

impl DiscoveryPlugin for TcpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        if !self.active.load(Ordering::Relaxed) {
            return None;
        }
        let host = self.detect_host()?;
        encode_endpoint(&host, self.cfg.rns_port, &self.cfg.endpoint_format)
    }

    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: Option<&[u8]>) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        let Some((host, port)) = parse_endpoint(payload, &self.cfg.endpoint_format) else {
            tracing::debug!(service = %self.cfg.service, "malformed discovery payload");
            return;
        };
        let name = format!("{}[{}]:{}", self.cfg.name_prefix, host, port);
        if self.attachments.holds(&name) {
            return;
        }
        let newcomer = quota::Peer {
            attached_as: name.clone(),
            estimate: self.estimate_for(announcer_pubkey),
        };
        match quota::judge(&self.attachments.kept(), newcomer, self.engine.path_count()) {
            quota::Verdict::Attach => {}
            quota::Verdict::Replace(displaced) => {
                tracing::info!(service = %self.cfg.service, %displaced, %name, "a nearer peer took an attached one's place");
                if let Some(gone) = self.attachments.release(&displaced) {
                    self.cap_controller.detach(gone.handle.id());
                }
            }
            quota::Verdict::Refuse => {
                tracing::debug!(service = %self.cfg.service, %name, "already attached to nearer and further peers");
                return;
            }
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
                self.attachments.hold(
                    name,
                    Attached {
                        service: self.cfg.service.clone(),
                        announced_by: who_announced(announcer_pubkey),
                        handle,
                    },
                );
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
    fn estimate_for(&self, announcer_pubkey: Option<&[u8]>) -> Option<Duration> {
        self.attachments
            .estimate_of(&who_announced(announcer_pubkey)?)
    }

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
