use std::sync::Mutex;

use tokio::sync::mpsc;

use crate::ble::radio::{ConnectionId, Outbound, PeerAddress, Radio, RadioError, RadioEvent};
use crate::ble::spec;

pub(super) struct Stub {
    pub(super) asked_to_read: Mutex<Vec<u128>>,
    outbound: mpsc::Sender<Outbound>,
    sent: Mutex<mpsc::Receiver<Outbound>>,
}

impl Default for Stub {
    fn default() -> Self {
        let (outbound, sent) = mpsc::channel(8);
        Self {
            asked_to_read: Mutex::new(Vec::new()),
            outbound,
            sent: Mutex::new(sent),
        }
    }
}

impl Stub {
    pub(super) fn what_went_out(&self) -> Vec<Vec<u8>> {
        let mut sent = self.sent.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        while let Ok(piece) = sent.try_recv() {
            out.push(piece.value);
        }
        out
    }
}

impl Radio for Stub {
    fn events_taken_once(&self) -> Option<mpsc::Receiver<RadioEvent>> {
        None
    }
    fn advertise(&self, _: &str, _: &[u8], _: u128) -> Result<(), RadioError> {
        Ok(())
    }
    fn stop_advertising(&self) {}
    fn scan(&self, _: u128) -> Result<(), RadioError> {
        Ok(())
    }
    fn stop_scan(&self) {}
    fn connect(&self, _: &PeerAddress) -> Result<(), RadioError> {
        Ok(())
    }
    fn disconnect(&self, _: ConnectionId) {}
    fn outbound_awaits_a_slot(&self) -> mpsc::Sender<Outbound> {
        self.outbound.clone()
    }
    fn read(&self, _: ConnectionId, characteristic: u128) -> Result<(), RadioError> {
        self.asked_to_read
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(characteristic);
        Ok(())
    }
    fn bytes_one_write_carries(&self, _: ConnectionId) -> usize {
        20
    }
    fn serve_identity(&self, _: [u8; spec::IDENTITY_LEN]) {}
}
