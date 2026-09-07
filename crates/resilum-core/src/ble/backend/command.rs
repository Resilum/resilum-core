use crate::ble::radio::{ConnectionId, PeerAddress};
use crate::ble::spec;

pub(super) enum Command {
    Advertise {
        local_name: String,
        beacon: Vec<u8>,
        service: u128,
    },
    StopAdvertising,
    Scan {
        service: u128,
    },
    StopScan,
    Connect(PeerAddress),
    Disconnect(ConnectionId),
    Read {
        conn: ConnectionId,
        characteristic: u128,
    },
    ServeIdentity([u8; spec::IDENTITY_LEN]),
}
