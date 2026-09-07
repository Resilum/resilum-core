use std::collections::HashMap;

use uuid::Uuid;

use super::super::{arrived, at, seen};
use super::{bench, their_address, them};
use crate::ble::radio::RadioEvent;
use crate::ble::spec;

#[tokio::test]
async fn a_beacon_in_service_data_reaches_the_node_that_scans() {
    let mut bench = bench();
    let advertised = HashMap::from([(Uuid::from_u128(spec::SERVICE), vec![1, 2, 3, 4])]);

    seen(&bench.told, at(&them()), Some("R".to_owned()), &advertised).await;

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

    seen(&bench.told, at(&them()), None, &someone_elses).await;

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
async fn a_notification_from_a_peer_we_never_met_is_let_go() {
    let mut bench = bench();

    arrived(&bench.told, &their_address(), spec::SERVICE, vec![1]).await;

    assert!(bench.heard.try_recv().is_err());
}
