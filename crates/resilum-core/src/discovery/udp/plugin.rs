use super::{PeerLeavesWhenDropped, UdpDiscovered};
use crate::discovery::DiscoveryPlugin;
use crate::discovery::admit::{self, Room};
use crate::discovery::attachments::Attached;
use crate::discovery::endpoint::{encode_endpoint, parse_endpoint};

impl DiscoveryPlugin for UdpDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        let inner = &self.inner;
        let (_, port) = inner.bound_to.rsplit_once(':')?;
        let reachable_on = inner.ours.effective().first()?;
        encode_endpoint(reachable_on, port.parse().ok()?, &inner.cfg.endpoint_format)
    }

    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: Option<&[u8]>) {
        let inner = &self.inner;
        let Some((host, port)) = parse_endpoint(payload, &inner.cfg.endpoint_format) else {
            tracing::debug!(service = %inner.cfg.service, "malformed discovery payload");
            return;
        };
        let peer = format!("{host}:{port}");
        let name = format!("{}[{peer}]", inner.cfg.name_prefix);
        if inner.attachments.holds(&name) {
            return;
        }
        let announced_by = admit::who_announced(announcer_pubkey.unwrap_or_default());
        let displaced = match admit::room_for(
            &inner.attachments,
            inner.engine.path_count(),
            &name,
            announced_by,
        ) {
            Room::Yes => None,
            Room::OnceThisIsLetGo(gone) => Some(gone),
            Room::No => return,
        };
        drop(displaced);

        if !inner
            .targets
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add(peer.clone())
        {
            return;
        }
        let Some(interface) = inner.rebuild() else {
            inner
                .targets
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .forget(&peer);
            return;
        };
        tracing::info!(%name, "attached discovered peer");
        inner
            .origin_registry
            .record(interface, inner.cfg.service.clone());
        inner.attachments.hold(
            name,
            Attached {
                service: inner.cfg.service.clone(),
                announced_by,
                interface,
                _detaches_when_dropped: Box::new(PeerLeavesWhenDropped {
                    inner: inner.clone(),
                    peer,
                }),
            },
        );
    }
}
