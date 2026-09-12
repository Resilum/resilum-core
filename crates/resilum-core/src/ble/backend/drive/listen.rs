mod telling;

use blew::central::CentralEvent;
use blew::peripheral::{PeripheralRequest, PeripheralStateEvent};
use blew::types::BleDevice;

use super::Reporting;
use crate::ble::radio::PeerAddress;
use crate::ble::spec;
use telling::{arrived, arrived_on, met_us, parted, seen};

pub(super) async fn what_the_central_heard(told: &mut Reporting, event: CentralEvent) {
    match event {
        CentralEvent::DeviceDiscovered(device) => {
            let BleDevice {
                id,
                name,
                services,
                service_data,
                ..
            } = device;
            seen(told, at(&id), name, &services, &service_data).await;
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

#[cfg(test)]
mod tests;
