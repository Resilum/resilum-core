use blew::central::{ScanFilter, WriteType};
use blew::peripheral::AdvertisingConfig;

use super::{ATT_HEADER, Held};
use crate::ble::radio::{ConnectionId, Outbound, PeerAddress, RadioEvent, Role};

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
            let _ = held.peripheral.stop_advertising().await;
        }
        Command::Scan { service } => {
            let filter = ScanFilter {
                services: vec![uuid::Uuid::from_u128(service)],
                ..ScanFilter::default()
            };
            let _ = held.central.start_scan(filter).await;
        }
        Command::StopScan => {
            let _ = held.central.stop_scan().await;
        }
        Command::Connect(address) => connect(held, address).await,
        Command::Disconnect(conn) => disconnect(held, conn).await,
        Command::Read {
            conn,
            characteristic,
        } => read(held, conn, characteristic).await,
        Command::ServeIdentity(identity) => {
            let _ = held
                .peripheral
                .add_service(&super::super::served::service(identity))
                .await;
        }
    }
}

async fn connect(held: &mut Held, address: PeerAddress) {
    let device = blew::DeviceId::from(address.0.clone());
    if held.central.connect(&device).await.is_err() {
        return;
    }
    let notified = uuid::Uuid::from_u128(super::super::spec::TX_NOTIFIED_BY_THE_PERIPHERAL);
    if let Err(error) = held
        .central
        .subscribe_characteristic(&device, notified)
        .await
    {
        tracing::warn!(%error, "the peer let us in but not to its notifications");
    }
    let conn = held.told.peers.joined(address.clone(), Role::Central);
    let carried = usize::from(held.central.mtu(&device).await).saturating_sub(ATT_HEADER);
    held.told.now_carries(conn, carried);
    let _ = held
        .told
        .telling
        .send(RadioEvent::Connected {
            conn,
            address,
            role: Role::Central,
            bytes_one_write_carries: carried,
        })
        .await;
}

async fn disconnect(held: &mut Held, conn: ConnectionId) {
    let Some(address) = held.told.peers.parted(conn) else {
        return;
    };
    held.told.forget_what_it_carried(conn);
    let device = blew::DeviceId::from(address.0);
    let _ = held.central.disconnect(&device).await;
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

pub(super) async fn put_on_the_air(held: &Held, piece: Outbound) {
    let Some((address, role)) = held
        .told
        .peers
        .address_of(piece.conn)
        .zip(held.told.peers.role_on(piece.conn))
    else {
        return;
    };
    let want = uuid::Uuid::from_u128(piece.characteristic);
    let device = blew::DeviceId::from(address.0);
    match role {
        Role::Central => {
            let _ = held
                .central
                .write_characteristic(&device, want, piece.value, how(piece.acknowledged))
                .await;
        }
        Role::Peripheral => {
            let _ = held
                .peripheral
                .notify_characteristic(&device, want, piece.value)
                .await;
        }
    }
}

fn how(acknowledged: bool) -> WriteType {
    if acknowledged {
        WriteType::WithResponse
    } else {
        WriteType::WithoutResponse
    }
}
