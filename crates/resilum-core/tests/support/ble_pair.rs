use std::sync::Arc;

use resilum_core::ble::framing::fragment;
use resilum_core::ble::radio::{ConnectionId, Outbound, Radio, RadioEvent, Role};
use resilum_core::ble::spec;
use tokio::sync::mpsc::Receiver;

use crate::fake_radio::{Air, CARRIED_PER_WRITE, FakeRadio};

pub const SERVED_IDENTITY: [u8; spec::IDENTITY_LEN] = [0x5A; spec::IDENTITY_LEN];

const BEFORE_A_PAIR_IS_GIVEN_UP_ON: std::time::Duration = std::time::Duration::from_secs(10);

pub struct Pair {
    pub dialer: Arc<FakeRadio>,
    pub answerer: Arc<FakeRadio>,
    pub there: ConnectionId,
    pub back: ConnectionId,
    pub heard_by_dialer: Receiver<RadioEvent>,
    _heard_by_answerer: Receiver<RadioEvent>,
}

pub async fn joined(air: &Air, tag: &str) -> Pair {
    let dialer = Arc::new(FakeRadio::on(air, &format!("aa:{tag}")));
    let answerer = Arc::new(FakeRadio::on(air, &format!("bb:{tag}")));
    let mut heard_by_dialer = dialer.events_taken_once().expect("stream");
    let mut heard_by_answerer = answerer.events_taken_once().expect("stream");

    answerer
        .advertise("Rtest", &[], spec::SERVICE)
        .expect("advert");
    answerer.serve_identity(SERVED_IDENTITY);
    dialer
        .connect(&resilum_core::ble::radio::PeerAddress(format!("bb:{tag}")))
        .expect("dial");

    let there = next_connection(&mut heard_by_dialer).await;
    let back = next_connection(&mut heard_by_answerer).await;
    Pair {
        dialer,
        answerer,
        there,
        back,
        heard_by_dialer,
        _heard_by_answerer: heard_by_answerer,
    }
}

pub async fn next_connection(events: &mut Receiver<RadioEvent>) -> ConnectionId {
    let waited = tokio::time::timeout(BEFORE_A_PAIR_IS_GIVEN_UP_ON, async {
        loop {
            if let RadioEvent::Connected { conn, .. } = events.recv().await.expect("an event") {
                return conn;
            }
        }
    });
    waited.await.expect("the pair never connected")
}

pub async fn put_on_the_air(
    air: &Air,
    radio: &FakeRadio,
    conn: ConnectionId,
    role: Role,
    packet: &[u8],
) {
    let outbound = radio.outbound_awaits_a_slot();
    let pieces = fragment(packet, CARRIED_PER_WRITE);
    let expected = air.wire().tapped.len() + pieces.len();
    for piece in pieces {
        outbound
            .send(Outbound {
                conn,
                characteristic: role.sends_on(),
                value: piece,
                acknowledged: false,
            })
            .await
            .expect("send");
    }
    for _ in 0..200 {
        if air.wire().tapped.len() >= expected {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the air never carried what was written to it");
}
