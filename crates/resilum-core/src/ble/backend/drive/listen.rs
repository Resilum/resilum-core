use std::collections::HashMap;

use blew::central::CentralEvent;
use blew::peripheral::{PeripheralRequest, PeripheralStateEvent};
use blew::types::BleDevice;
use uuid::Uuid;

use super::{ATT_HEADER, Reporting};
use crate::ble::radio::{ConnectionId, PeerAddress, RadioEvent, Role};
use crate::ble::spec;

const SMALLEST_MTU: usize = 23;

pub(super) async fn what_the_central_heard(told: &mut Reporting, event: CentralEvent) {
    match event {
        CentralEvent::DeviceDiscovered(device) => {
            let BleDevice {
                id,
                name,
                service_data,
                ..
            } = device;
            seen(told, at(&id), name, &service_data).await;
        }
        CentralEvent::CharacteristicNotification {
            device_id,
            char_uuid,
            value,
        } => arrived(told, &at(&device_id), char_uuid.as_u128(), value.to_vec()).await,
        CentralEvent::DeviceDisconnected { device_id, .. } => parted(told, &at(&device_id)).await,
        CentralEvent::DeviceConnected { .. } | CentralEvent::AdapterStateChanged { .. } => {}
    }
}

pub(super) async fn what_the_peripheral_saw(told: &mut Reporting, event: PeripheralStateEvent) {
    match event {
        PeripheralStateEvent::SubscriptionChanged {
            client_id,
            char_uuid,
            subscribed,
        } if char_uuid.as_u128() == spec::TX_NOTIFIED_BY_THE_PERIPHERAL => {
            let address = at(&client_id);
            if subscribed {
                met_us(told, address).await;
            } else {
                parted(told, &address).await;
            }
        }
        PeripheralStateEvent::SubscriptionChanged { .. }
        | PeripheralStateEvent::AdapterStateChanged { .. } => {}
    }
}

pub(super) async fn what_a_central_asked(told: &mut Reporting, request: PeripheralRequest) {
    match request {
        PeripheralRequest::Write {
            client_id,
            char_uuid,
            value,
            responder,
            ..
        } => {
            if let Some(responder) = responder {
                responder.success();
            }
            let conn = met_us(told, at(&client_id)).await;
            arrived_on(told, conn, char_uuid.as_u128(), value).await;
        }
        PeripheralRequest::Read { .. } => {}
    }
}

fn at(device_id: &blew::DeviceId) -> PeerAddress {
    PeerAddress(device_id.as_str().to_owned())
}

async fn seen(
    told: &Reporting,
    address: PeerAddress,
    name: Option<String>,
    service_data: &HashMap<Uuid, Vec<u8>>,
) {
    let ours = Uuid::from_u128(spec::SERVICE);
    let beacon = service_data.get(&ours).cloned().unwrap_or_default();
    let _ = told
        .telling
        .send(RadioEvent::Seen {
            address,
            name,
            beacon,
        })
        .await;
}

async fn arrived(told: &Reporting, address: &PeerAddress, characteristic: u128, value: Vec<u8>) {
    let Some(conn) = told.peers.conn_at(address) else {
        return;
    };
    arrived_on(told, conn, characteristic, value).await;
}

async fn arrived_on(told: &Reporting, conn: ConnectionId, characteristic: u128, value: Vec<u8>) {
    let _ = told
        .telling
        .send(RadioEvent::Data {
            conn,
            characteristic,
            value,
        })
        .await;
}

async fn parted(told: &mut Reporting, address: &PeerAddress) {
    let Some(conn) = told.peers.conn_at(address) else {
        return;
    };
    told.peers.parted(conn);
    told.forget_what_it_carried(conn);
    let _ = told.telling.send(RadioEvent::Disconnected { conn }).await;
}

async fn met_us(told: &mut Reporting, address: PeerAddress) -> ConnectionId {
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

#[cfg(test)]
mod tests;
