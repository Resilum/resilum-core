use blew::peripheral::{PeripheralRequest, PeripheralStateEvent};
use uuid::Uuid;

use super::super::{met_us, parted, what_a_central_asked, what_the_peripheral_saw};
use super::{bench, their_address, them};
use crate::ble::radio::{RadioEvent, Role};
use crate::ble::spec;

fn subscription(char_uuid: u128, subscribed: bool) -> PeripheralStateEvent {
    PeripheralStateEvent::SubscriptionChanged {
        client_id: them(),
        char_uuid: Uuid::from_u128(char_uuid),
        subscribed,
    }
}

fn a_write(value: Vec<u8>) -> PeripheralRequest {
    PeripheralRequest::Write {
        client_id: them(),
        service_uuid: Uuid::from_u128(spec::SERVICE),
        char_uuid: Uuid::from_u128(spec::RX_WRITTEN_BY_THE_CENTRAL),
        offset: 0,
        value,
        responder: None,
    }
}

#[tokio::test]
async fn a_central_that_subscribes_is_a_connection_we_never_dialled() {
    let mut bench = bench();

    what_the_peripheral_saw(
        &mut bench.told,
        subscription(spec::TX_NOTIFIED_BY_THE_PERIPHERAL, true),
    )
    .await;

    let Some(RadioEvent::Connected { conn, role, .. }) = bench.heard.recv().await else {
        panic!("a central subscribed and nobody was told");
    };
    assert_eq!(role, Role::Peripheral);
    assert_eq!(bench.carried.lock().expect("held").get(&conn), Some(&20));
}

#[tokio::test]
async fn subscribing_to_anything_else_is_not_a_connection() {
    let mut bench = bench();

    what_the_peripheral_saw(
        &mut bench.told,
        subscription(spec::IDENTITY_READ_FROM_THE_PERIPHERAL, true),
    )
    .await;

    assert!(bench.heard.try_recv().is_err());
}

#[tokio::test]
async fn a_central_that_writes_before_it_subscribes_is_a_connection_all_the_same() {
    let mut bench = bench();

    what_a_central_asked(&mut bench.told, a_write(vec![7, 7])).await;

    assert!(matches!(
        bench.heard.recv().await,
        Some(RadioEvent::Connected { .. })
    ));
    let Some(RadioEvent::Data {
        characteristic,
        value,
        ..
    }) = bench.heard.recv().await
    else {
        panic!("the write never became data");
    };
    assert_eq!(characteristic, spec::RX_WRITTEN_BY_THE_CENTRAL);
    assert_eq!(value, vec![7, 7]);
}

#[tokio::test]
async fn a_peer_that_drops_is_forgotten_and_stops_carrying_anything() {
    let mut bench = bench();
    let conn = met_us(&mut bench.told, their_address()).await;
    let _ = bench.heard.recv().await;

    parted(&mut bench.told, &their_address()).await;

    assert_eq!(
        bench.heard.recv().await,
        Some(RadioEvent::Disconnected { conn })
    );
    assert_eq!(bench.carried.lock().expect("held").get(&conn), None);
}

#[tokio::test]
async fn a_central_seen_twice_keeps_the_connection_it_already_had() {
    let mut bench = bench();

    let first = met_us(&mut bench.told, their_address()).await;
    let again = met_us(&mut bench.told, their_address()).await;

    assert_eq!(first, again);
    assert!(matches!(
        bench.heard.recv().await,
        Some(RadioEvent::Connected { .. })
    ));
    assert!(bench.heard.try_recv().is_err());
}
