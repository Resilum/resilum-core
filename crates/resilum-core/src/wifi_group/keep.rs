use tokio::net::{TcpListener, TcpStream};

use super::{ATTACHED_AS, Links, Ours};
use crate::discovery::admit::{self, Reached, Room};
use crate::discovery::attachments::Attached;

pub(super) async fn whoever_joins(ours: Ours, links: Links, listener: TcpListener) {
    while let Ok((stream, _)) = listener.accept().await {
        this_peer(&ours, &links, stream);
    }
}

pub(super) fn this_peer(ours: &Ours, links: &Links, stream: TcpStream) {
    let Some(name) = named(&stream) else {
        return;
    };
    if ours.attachments.holds(&name) {
        return;
    }
    match admit::room_for(
        &ours.attachments,
        ours.engine.path_count(),
        &name,
        None,
        Reached::OverTheRadio,
    ) {
        Room::Yes | Room::OnceThisIsLetGo(_) => {}
        Room::No => return,
    }
    match ours.engine.spawn_byte_channel(&name, stream) {
        Ok(handle) => {
            ours.origins.record(handle.id(), ATTACHED_AS);
            ours.attachments.hold(
                name.clone(),
                Attached {
                    service: ATTACHED_AS.to_owned(),
                    announced_by: None,
                    interface: handle.id(),
                    _detaches_when_dropped: Box::new(()),
                },
            );
            links
                .lock()
                .unwrap_or_else(|held| held.into_inner())
                .insert(name.clone(), handle);
            tracing::info!(%name, "attached a peer over the wi-fi group");
        }
        Err(e) => tracing::warn!(%name, error = %e, "wi-fi group byte-channel attach failed"),
    }
}

fn named(stream: &TcpStream) -> Option<String> {
    match stream.peer_addr() {
        Ok(peer) => Some(format!("WifiGroup[{}]", peer.ip())),
        Err(e) => {
            tracing::warn!(error = %e, "a wi-fi group socket named no peer");
            None
        }
    }
}
