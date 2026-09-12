//! Announcing our own address, and dialling the peers who announce theirs.

use std::sync::atomic::Ordering;

use super::{Attached, TcpDiscovered};
use crate::config::SocksProxy;
use crate::discovery::DiscoveryPlugin;
use crate::discovery::admit::{self, Room};
use crate::discovery::endpoint::{encode_endpoint, parse_endpoint};

impl DiscoveryPlugin for TcpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        if !self.transport_is_up.load(Ordering::Relaxed) {
            return None;
        }
        let host = self.detect_host()?;
        encode_endpoint(&host, self.cfg.rns_port, &self.cfg.endpoint_format)
    }

    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: Option<&[u8]>) {
        if !self.transport_is_up.load(Ordering::Relaxed) {
            return;
        }
        let Some((host, port)) = parse_endpoint(payload, &self.cfg.endpoint_format) else {
            tracing::debug!(service = %self.cfg.service, "malformed discovery payload");
            return;
        };
        let name = format!("{}[{}]:{}", self.cfg.name_prefix, host, port);
        let peer = admit::who_announced(announcer_pubkey.unwrap_or_default());
        if self.attachments.holds(&name) {
            if let Some(peer) = peer
                && self.attachments.learn_who_announced(&name, peer)
            {
                tracing::debug!(service = %self.cfg.service, %name, "a peer attached from cache has a name now");
            }
            return;
        }
        match admit::room_for(
            &self.attachments,
            self.engine.path_count(),
            &name,
            peer,
            admit::Reached::OverTheNetwork,
        ) {
            Room::Yes => {}
            Room::OnceThisIsLetGo(gone) => self.cap_controller.detach(gone.interface),
            Room::No => return,
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
                        announced_by: peer,
                        interface: handle.id(),
                        _detaches_when_dropped: Box::new(handle),
                    },
                );
                self.announce_again.notify_waiters();
                self.remembered.seen(payload, crate::wall_clock::unix_now());
            }
            Err(e) => {
                tracing::warn!(service = %self.cfg.service, %name, error = %e, "attach failed");
            }
        }
    }

    fn forget_stale_peers(&self, now: f64) {
        self.remembered.forget_stale(now);
    }
}
