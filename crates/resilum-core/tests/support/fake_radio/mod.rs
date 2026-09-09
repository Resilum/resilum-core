mod air;

pub use air::Air;

pub const CARRIED_PER_WRITE: usize = 20;

use std::sync::Mutex;

use tokio::sync::mpsc;

use resilum_core::ble::radio::{
    ConnectionId, Outbound, PeerAddress, Radio, RadioError, RadioEvent,
};
use resilum_core::ble::spec;

const DEEP_ENOUGH_FOR_A_TEST: usize = 256;

pub struct FakeRadio {
    address: PeerAddress,
    air: Air,
    to_us: mpsc::Sender<RadioEvent>,
    ours: Mutex<Option<mpsc::Receiver<RadioEvent>>>,
    outbound: mpsc::Sender<Outbound>,
    the_name_is_ours: bool,
}

impl FakeRadio {
    #[must_use]
    pub fn on(air: &Air, address: &str) -> Self {
        let (to_us, ours) = mpsc::channel(DEEP_ENOUGH_FOR_A_TEST);
        let (outbound, waiting) = mpsc::channel(DEEP_ENOUGH_FOR_A_TEST);
        let address = PeerAddress(address.to_owned());
        std::thread::spawn({
            let air = air.clone();
            let ours = address.clone();
            move || deliver_to_the_far_end(&air, &ours, waiting)
        });
        Self {
            address,
            air: air.clone(),
            to_us,
            ours: Mutex::new(Some(ours)),
            outbound,
            the_name_is_ours: true,
        }
    }
}

impl Radio for FakeRadio {
    fn events_taken_once(&self) -> Option<mpsc::Receiver<RadioEvent>> {
        self.ours.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn advertise(&self, local_name: &str, beacon: &[u8], _service: u128) -> Result<(), RadioError> {
        self.air
            .advertise(&self.address, local_name, beacon, self.to_us.clone());
        Ok(())
    }

    fn stop_advertising(&self) {}

    fn the_local_name_is_ours_to_spend(&self) -> bool {
        self.the_name_is_ours
    }

    fn scan(&self, _service: u128) -> Result<(), RadioError> {
        for seen in self.air.seen_by(&self.address) {
            self.to_us.try_send(seen).map_err(queue_full)?;
        }
        Ok(())
    }

    fn stop_scan(&self) {}

    fn connect(&self, address: &PeerAddress) -> Result<(), RadioError> {
        self.air
            .join(&self.address, self.to_us.clone(), address)
            .ok_or(RadioError::NotConnected)?;
        Ok(())
    }

    fn disconnect(&self, conn: ConnectionId) {
        self.air.part(conn);
    }

    fn outbound_awaits_a_slot(&self) -> mpsc::Sender<Outbound> {
        self.outbound.clone()
    }

    fn read(&self, conn: ConnectionId, characteristic: u128) -> Result<(), RadioError> {
        let far = self
            .air
            .far_end(conn, &self.address)
            .ok_or(RadioError::NotConnected)?;
        let value = self
            .air
            .identity_of(&far.address)
            .ok_or(RadioError::Unsupported)?;
        self.to_us
            .try_send(RadioEvent::Data {
                conn,
                characteristic,
                value: value.to_vec(),
            })
            .map_err(queue_full)
    }

    fn bytes_one_write_carries(&self, _conn: ConnectionId) -> usize {
        self.air.wire().carried_per_write
    }

    fn serve_identity(&self, identity: [u8; spec::IDENTITY_LEN]) {
        self.air.serve_identity(&self.address, identity);
    }
}

fn deliver_to_the_far_end(air: &Air, ours: &PeerAddress, mut waiting: mpsc::Receiver<Outbound>) {
    while let Some(piece) = waiting.blocking_recv() {
        let Some(far) = air.far_end(piece.conn, ours) else {
            continue;
        };
        air.wire()
            .tapped
            .push((piece.conn, piece.characteristic, piece.value.clone()));
        let _ = far.to.blocking_send(RadioEvent::Data {
            conn: piece.conn,
            characteristic: piece.characteristic,
            value: piece.value,
        });
    }
}

fn queue_full<T>(why: mpsc::error::TrySendError<T>) -> RadioError {
    match why {
        mpsc::error::TrySendError::Full(_) => RadioError::QueueFull,
        mpsc::error::TrySendError::Closed(_) => RadioError::NotConnected,
    }
}
