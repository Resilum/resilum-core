use super::Ours;
use crate::ble::handshake::{Handshakes, Told};
use crate::ble::link::PeerLink;
use crate::ble::links::{Kept, Links};
use crate::ble::radio::ConnectionId;
use crate::discovery::admit::{self, Room};
use crate::discovery::attachments::Attached;

pub(super) fn on_handshake(
    ours: &Ours,
    links: &mut Links,
    waiting: &mut Handshakes,
    conn: ConnectionId,
    characteristic: u128,
    value: &[u8],
    now_ms: u64,
) {
    let told = waiting.heard(
        ours.radio.as_ref(),
        conn,
        characteristic,
        value,
        ours.identity,
    );
    let Told::ThisIsTheirIdentity(peer, met) = told else {
        if matches!(told, Told::NotOneOfOurs) {
            ours.radio.disconnect(conn);
        }
        return;
    };
    let name = format!("BleDiscovered[{}]", crate::hex::encode(peer.iter()));
    if ours.attachments.holds(&name) {
        ours.radio.disconnect(conn);
        return;
    }
    match admit::room_for(
        &ours.attachments,
        ours.engine.path_count(),
        &name,
        Some(peer),
        admit::Reached::OverTheRadio,
    ) {
        Room::Yes | Room::OnceThisIsLetGo(_) => {}
        Room::No => {
            ours.radio.disconnect(conn);
            return;
        }
    }
    let opened = PeerLink::open(
        &ours.engine,
        &name,
        ours.radio.clone(),
        conn,
        met.role,
        peer,
        || 0,
    );
    match opened {
        Ok(link) => keep(ours, links, name, link, conn, now_ms),
        Err(e) => {
            tracing::warn!(%name, error = %e, "ble attach failed");
            ours.radio.disconnect(conn);
        }
    }
}

fn keep(
    ours: &Ours,
    links: &mut Links,
    name: String,
    link: PeerLink,
    conn: ConnectionId,
    now_ms: u64,
) {
    let interface = link.interface;
    let peer = link.peer;
    match links.keep_unless_already_reached(link) {
        Kept::AlreadyReachedThatWay(_) => ours.radio.disconnect(conn),
        Kept::Yes => {
            ours.field.met_over_the_radio(peer, now_ms);
            ours.origins.record(interface, crate::ble::ATTACHED_AS);
            ours.attachments.hold(
                name.clone(),
                Attached {
                    service: crate::ble::ATTACHED_AS.to_owned(),
                    announced_by: Some(peer),
                    interface,
                    _detaches_when_dropped: Box::new(()),
                },
            );
            tracing::info!(%name, "ble peer attached");
        }
    }
}
