use tokio::sync::mpsc;

use super::spec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerAddress(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Central,
    Peripheral,
}

impl Role {
    #[must_use]
    pub fn sends_on(self) -> u128 {
        match self {
            Self::Central => spec::RX_WRITTEN_BY_THE_CENTRAL,
            Self::Peripheral => spec::TX_NOTIFIED_BY_THE_PERIPHERAL,
        }
    }

    #[must_use]
    pub fn hears_on(self) -> u128 {
        match self {
            Self::Central => spec::TX_NOTIFIED_BY_THE_PERIPHERAL,
            Self::Peripheral => spec::RX_WRITTEN_BY_THE_CENTRAL,
        }
    }

    #[must_use]
    pub fn theirs(self) -> Self {
        match self {
            Self::Central => Self::Peripheral,
            Self::Peripheral => Self::Central,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadioEvent {
    Seen {
        address: PeerAddress,
        name: Option<String>,
    },
    Connected {
        conn: ConnectionId,
        address: PeerAddress,
        role: Role,
        bytes_one_write_carries: usize,
    },
    Data {
        conn: ConnectionId,
        characteristic: u128,
        value: Vec<u8>,
    },
    WritableChanged {
        conn: ConnectionId,
        bytes_one_write_carries: usize,
    },
    Disconnected {
        conn: ConnectionId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outbound {
    pub conn: ConnectionId,
    pub characteristic: u128,
    pub value: Vec<u8>,
    pub acknowledged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadioError {
    NotConnected,
    QueueFull,
    Unsupported,
    Backend(String),
}

pub trait Radio: Send + Sync + 'static {
    fn events_taken_once(&self) -> Option<mpsc::Receiver<RadioEvent>>;

    fn advertise(&self, local_name: &str, service: u128) -> Result<(), RadioError>;
    fn stop_advertising(&self);

    fn scan(&self, service: u128) -> Result<(), RadioError>;
    fn stop_scan(&self);

    fn connect(&self, address: &PeerAddress) -> Result<(), RadioError>;
    fn disconnect(&self, conn: ConnectionId);

    fn outbound_awaits_a_slot(&self) -> mpsc::Sender<Outbound>;
    fn read(&self, conn: ConnectionId, characteristic: u128) -> Result<(), RadioError>;

    fn bytes_one_write_carries(&self, conn: ConnectionId) -> usize;
    fn serve_identity(&self, identity: [u8; spec::IDENTITY_LEN]);
}
