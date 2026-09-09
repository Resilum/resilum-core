use std::collections::HashMap;

use uuid::Uuid;

use crate::ble::backend::drive::{ATT_HEADER, Reporting};
use crate::ble::radio::{ConnectionId, PeerAddress, RadioEvent, Role};
use crate::ble::spec;

const SMALLEST_MTU: usize = 23;

pub(super) async fn seen(
    told: &Reporting,
    address: PeerAddress,
    name: Option<String>,
    services: &[Uuid],
    service_data: &HashMap<Uuid, Vec<u8>>,
) {
    let ours = Uuid::from_u128(spec::SERVICE);
    if !services.contains(&ours) {
        return;
    }
    let beacon = service_data.get(&ours).cloned().unwrap_or_default();
    tracing::debug!(
        peer = %address.0,
        named = name.is_some(),
        beacon_bytes = beacon.len(),
        keys = service_data.len(),
        "heard one of ours on the air"
    );
    let _ = told
        .telling
        .send(RadioEvent::Seen {
            address,
            name,
            beacon,
        })
        .await;
}

pub(super) async fn arrived(
    told: &Reporting,
    address: &PeerAddress,
    characteristic: u128,
    value: Vec<u8>,
) {
    let Some(conn) = told.peers.conn_at(address) else {
        return;
    };
    arrived_on(told, conn, characteristic, value).await;
}

pub(super) async fn arrived_on(
    told: &Reporting,
    conn: ConnectionId,
    characteristic: u128,
    value: Vec<u8>,
) {
    let _ = told
        .telling
        .send(RadioEvent::Data {
            conn,
            characteristic,
            value,
        })
        .await;
}

pub(super) async fn parted(told: &mut Reporting, address: &PeerAddress) {
    let Some(conn) = told.peers.conn_at(address) else {
        return;
    };
    told.peers.parted(conn);
    told.forget_what_it_carried(conn);
    let _ = told.telling.send(RadioEvent::Disconnected { conn }).await;
}

pub(super) async fn met_us(told: &mut Reporting, address: PeerAddress) -> ConnectionId {
    if let Some(known) = told.peers.conn_at(&address) {
        return known;
    }
    let conn = told.peers.joined(address.clone(), Role::Peripheral);
    let carried = SMALLEST_MTU - ATT_HEADER;
    told.now_carries(conn, carried);
    let _ = told
        .telling
        .send(RadioEvent::Connected {
            conn,
            address,
            role: Role::Peripheral,
            bytes_one_write_carries: carried,
        })
        .await;
    conn
}
