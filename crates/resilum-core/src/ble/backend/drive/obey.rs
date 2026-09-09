use blew::central::ScanFilter;
use blew::peripheral::AdvertisingConfig;

use super::Held;
use super::dialling;
use crate::ble::radio::{ConnectionId, RadioEvent};

pub(super) async fn command(held: &mut Held, command: super::Command) {
    use super::Command;
    match command {
        Command::Advertise {
            local_name,
            beacon,
            service,
        } => {
            let ours = uuid::Uuid::from_u128(service);
            let config = AdvertisingConfig {
                local_name,
                service_uuids: vec![ours],
                service_data: (!beacon.is_empty())
                    .then_some((ours, beacon))
                    .into_iter()
                    .collect(),
            };
            if let Err(error) = held.peripheral.start_advertising(&config).await {
                tracing::warn!(%error, "the radio refused to advertise us");
            }
        }
        Command::StopAdvertising => {
            if let Err(error) = held.peripheral.stop_advertising().await {
                tracing::warn!(%error, "the radio kept advertising us");
            }
        }
        Command::Scan { service } => look_around(held, service).await,
        Command::StopScan => stop_looking(held).await,
        Command::Connect(address) => dialling::connect(held, address),
        Command::Disconnect(conn) => disconnect(held, conn).await,
        Command::Read {
            conn,
            characteristic,
        } => read(held, conn, characteristic).await,
        Command::ServeIdentity(identity) => {
            if let Err(error) = held
                .peripheral
                .add_service(&super::super::served::service(identity))
                .await
            {
                tracing::error!(%error, "the radio serves nothing: no peer can reach us");
            }
        }
    }
}

async fn look_around(held: &Held, service: u128) {
    let filter = ScanFilter {
        services: vec![uuid::Uuid::from_u128(service)],
        ..ScanFilter::default()
    };
    if let Err(error) = held.central.start_scan(filter).await {
        tracing::warn!(%error, "the radio refused to look around");
    }
}

async fn stop_looking(held: &Held) {
    if let Err(error) = held.central.stop_scan().await {
        tracing::warn!(%error, "the radio kept looking around");
    }
}

async fn disconnect(held: &mut Held, conn: ConnectionId) {
    let Some(address) = held.told.peers.parted(conn) else {
        return;
    };
    held.told.forget_what_it_carried(conn);
    let device = blew::DeviceId::from(address.0);
    if let Err(error) = held.central.disconnect(&device).await {
        tracing::debug!(%error, "the radio had already let the peer go");
    }
    let _ = held
        .told
        .telling
        .send(RadioEvent::Disconnected { conn })
        .await;
}

async fn read(held: &Held, conn: ConnectionId, characteristic: u128) {
    let Some(address) = held.told.peers.address_of(conn) else {
        return;
    };
    let device = blew::DeviceId::from(address.0);
    let want = uuid::Uuid::from_u128(characteristic);
    if let Ok(value) = held.central.read_characteristic(&device, want).await {
        let _ = held
            .told
            .telling
            .send(RadioEvent::Data {
                conn,
                characteristic,
                value: value.to_vec(),
            })
            .await;
    }
}
