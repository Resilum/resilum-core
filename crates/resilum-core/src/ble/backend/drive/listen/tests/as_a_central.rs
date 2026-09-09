use std::collections::HashMap;

use uuid::Uuid;

use super::super::{arrived, at, seen};
use super::{bench, their_address, them};
use crate::ble::radio::RadioEvent;
use crate::ble::spec;

fn ours() -> Vec<Uuid> {
    vec![Uuid::from_u128(spec::SERVICE)]
}

#[tokio::test]
async fn a_beacon_in_service_data_reaches_the_node_that_scans() {
    let mut bench = bench();
    let advertised = HashMap::from([(Uuid::from_u128(spec::SERVICE), vec![1, 2, 3, 4])]);

    seen(
        &bench.told,
        at(&them()),
        Some("R".to_owned()),
        &ours(),
        &advertised,
    )
    .await;

    assert_eq!(
        bench.heard.recv().await,
        Some(RadioEvent::Seen {
            address: their_address(),
            name: Some("R".to_owned()),
            beacon: vec![1, 2, 3, 4],
        })
    );
}

#[tokio::test]
async fn a_neighbour_carrying_no_service_data_of_ours_is_still_reported() {
    let mut bench = bench();
    let someone_elses = HashMap::from([(Uuid::from_u128(1), vec![9])]);

    seen(&bench.told, at(&them()), None, &ours(), &someone_elses).await;

    assert_eq!(
        bench.heard.recv().await,
        Some(RadioEvent::Seen {
            address: their_address(),
            name: None,
            beacon: Vec::new(),
        })
    );
}

#[tokio::test]
async fn a_device_that_does_not_advertise_our_service_is_no_neighbour_of_ours() {
    let mut bench = bench();
    let a_pair_of_headphones = vec![Uuid::from_u128(0x0000_110b_0000_1000_8000_0080_5f9b_34fb)];

    seen(
        &bench.told,
        at(&them()),
        Some("Headphones".to_owned()),
        &a_pair_of_headphones,
        &HashMap::new(),
    )
    .await;

    assert!(
        bench.heard.try_recv().is_err(),
        "the scan filter is advisory on BlueZ, so dialling what it lets through reaches strangers"
    );
}

#[tokio::test]
async fn a_notification_from_a_peer_we_never_met_is_let_go() {
    let mut bench = bench();

    arrived(&bench.told, &their_address(), spec::SERVICE, vec![1]).await;

    assert!(bench.heard.try_recv().is_err());
}
