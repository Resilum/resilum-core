mod command;
mod drive;
mod served;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use super::radio::{ConnectionId, Outbound, PeerAddress, Radio, RadioError, RadioEvent};
use super::spec;
use command::Command;

const COMMANDS_IN_FLIGHT: usize = 64;
const FRAGMENTS_IN_FLIGHT: usize = 32;
const EVENTS_IN_FLIGHT: usize = 256;

pub struct BlewRadio {
    commands: mpsc::Sender<Command>,
    outbound: mpsc::Sender<Outbound>,
    events: Mutex<Option<mpsc::Receiver<RadioEvent>>>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
}

impl BlewRadio {
    pub async fn open_or_say_why() -> Result<Self, RadioError> {
        let (commands, taking) = mpsc::channel(COMMANDS_IN_FLIGHT);
        let (outbound, waiting) = mpsc::channel(FRAGMENTS_IN_FLIGHT);
        let (telling, events) = mpsc::channel(EVENTS_IN_FLIGHT);
        let carried = Arc::new(Mutex::new(HashMap::new()));
        drive::spawn(taking, waiting, telling, Arc::clone(&carried)).await?;
        Ok(Self {
            commands,
            outbound,
            events: Mutex::new(Some(events)),
            carried,
        })
    }

    fn ask(&self, what: Command) -> Result<(), RadioError> {
        self.commands.try_send(what).map_err(|why| match why {
            mpsc::error::TrySendError::Full(_) => RadioError::QueueFull,
            mpsc::error::TrySendError::Closed(_) => RadioError::NotConnected,
        })
    }
}

impl Radio for BlewRadio {
    fn events_taken_once(&self) -> Option<mpsc::Receiver<RadioEvent>> {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn advertise(&self, local_name: &str, service: u128) -> Result<(), RadioError> {
        self.ask(Command::Advertise {
            local_name: local_name.to_owned(),
            service,
        })
    }

    fn stop_advertising(&self) {
        let _ = self.ask(Command::StopAdvertising);
    }

    fn scan(&self, service: u128) -> Result<(), RadioError> {
        self.ask(Command::Scan { service })
    }

    fn stop_scan(&self) {
        let _ = self.ask(Command::StopScan);
    }

    fn connect(&self, address: &PeerAddress) -> Result<(), RadioError> {
        self.ask(Command::Connect(address.clone()))
    }

    fn disconnect(&self, conn: ConnectionId) {
        let _ = self.ask(Command::Disconnect(conn));
    }

    fn outbound_awaits_a_slot(&self) -> mpsc::Sender<Outbound> {
        self.outbound.clone()
    }

    fn read(&self, conn: ConnectionId, characteristic: u128) -> Result<(), RadioError> {
        self.ask(Command::Read {
            conn,
            characteristic,
        })
    }

    fn bytes_one_write_carries(&self, conn: ConnectionId) -> usize {
        self.carried
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&conn)
            .copied()
            .unwrap_or(spec::FRAGMENT_HEADER_LEN)
    }

    fn serve_identity(&self, identity: [u8; spec::IDENTITY_LEN]) {
        let _ = self.ask(Command::ServeIdentity(identity));
    }
}
