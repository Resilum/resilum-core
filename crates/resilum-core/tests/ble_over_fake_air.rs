#[path = "support/ble_pair.rs"]
mod ble_pair;
#[path = "support/fake_radio/mod.rs"]
mod fake_radio;

use std::time::Duration;

use ble_pair::{SERVED_IDENTITY, joined, put_on_the_air};
use fake_radio::{Air, CARRIED_PER_WRITE};
use resilum_core::ble::framing::{Arrived, Reassembly};
use resilum_core::ble::radio::{Radio, RadioEvent, Role};
use resilum_core::ble::spec;

#[tokio::test]
async fn a_packet_written_on_one_side_arrives_whole_on_the_other() {
    let air = Air::new(CARRIED_PER_WRITE);
    let pair = joined(&air, "01").await;
    let packet: Vec<u8> = (0..200u32)
        .map(|n| u8::try_from(n % 251).unwrap_or(0))
        .collect();

    put_on_the_air(&air, &pair.dialer, pair.there, Role::Central, &packet).await;

    let mut held = Reassembly::default();
    let arrived = air
        .wire()
        .tapped
        .clone()
        .into_iter()
        .filter_map(
            |(_, _, piece)| match held.process(&piece, CARRIED_PER_WRITE, 0) {
                Arrived::Packet(whole) => Some(whole),
                _ => None,
            },
        )
        .last();

    assert_eq!(arrived, Some(packet));
}

#[tokio::test]
async fn what_lands_on_the_air_is_fragments_and_never_hdlc() {
    let air = Air::new(CARRIED_PER_WRITE);
    let pair = joined(&air, "02").await;

    put_on_the_air(
        &air,
        &pair.dialer,
        pair.there,
        Role::Central,
        &[0x7E, 0x7D, 0x01],
    )
    .await;

    let tapped = air.wire().tapped.clone();
    assert!(!tapped.is_empty());
    for (_, characteristic, piece) in tapped {
        assert_eq!(characteristic, spec::RX_WRITTEN_BY_THE_CENTRAL);
        assert!(
            piece[0] <= spec::TYPE_END,
            "{piece:?} does not start a fragment"
        );
        assert!(piece.len() <= CARRIED_PER_WRITE);
    }
}

#[tokio::test]
async fn a_peripheral_answers_where_a_central_is_listening() {
    let air = Air::new(CARRIED_PER_WRITE);
    let pair = joined(&air, "03").await;

    put_on_the_air(&air, &pair.answerer, pair.back, Role::Peripheral, &[0xAB]).await;

    let (_, characteristic, _) = air.wire().tapped.last().cloned().expect("a write");
    assert_eq!(characteristic, spec::TX_NOTIFIED_BY_THE_PERIPHERAL);
    assert_eq!(characteristic, Role::Central.hears_on());
}

#[tokio::test]
async fn a_central_reads_the_identity_the_peripheral_serves() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut pair = joined(&air, "04").await;

    pair.dialer
        .read(pair.there, spec::IDENTITY_READ_FROM_THE_PERIPHERAL)
        .expect("read");

    let value = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let RadioEvent::Data { value, .. } =
                pair.heard_by_dialer.recv().await.expect("an event")
            {
                return value;
            }
        }
    })
    .await
    .expect("the identity arrives");

    assert_eq!(value, SERVED_IDENTITY.to_vec());
}

#[tokio::test(start_paused = true)]
#[should_panic(expected = "the pair never connected")]
async fn a_pair_that_never_joins_gives_up_rather_than_waiting_forever() {
    let (_never_sends, mut heard) = tokio::sync::mpsc::channel(1);

    ble_pair::next_connection(&mut heard).await;
}
