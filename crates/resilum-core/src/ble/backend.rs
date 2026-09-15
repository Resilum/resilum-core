use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use self::command::Command;
use super::radio::{ConnectionId, Outbound, PeerAddress, Radio, RadioError, RadioEvent};
use super::spec;
use crate::letting_go::OnTheWayOut as _;

mod command;
mod drive;
mod served;
#[cfg(target_os = "android")]
mod the_jvm;

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
        #[cfg(target_os = "android")]
        the_jvm::hand_it_to_the_radio();
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

    fn advertise(&self, local_name: &str, beacon: &[u8], service: u128) -> Result<(), RadioError> {
        self.ask(Command::Advertise {
            local_name: local_name.to_owned(),
            beacon: beacon.to_vec(),
            service,
        })
    }

    fn stop_advertising(&self) {
        self.ask(Command::StopAdvertising)
            .on_the_way_out("the order to stop advertising");
    }

    fn the_local_name_is_ours_to_spend(&self) -> bool {
        !cfg!(target_os = "android")
    }

    fn scan(&self, service: u128) -> Result<(), RadioError> {
        self.ask(Command::Scan { service })
    }

    fn stop_scan(&self) {
        self.ask(Command::StopScan)
            .on_the_way_out("the order to stop scanning");
    }

    fn connect(&self, address: &PeerAddress) -> Result<(), RadioError> {
        self.ask(Command::Connect(address.clone()))
    }

    fn disconnect(&self, conn: ConnectionId) {
        self.ask(Command::Disconnect(conn))
            .on_the_way_out("the order to drop a connection");
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
        if let Err(error) = self.ask(Command::ServeIdentity(identity)) {
            tracing::warn!(%error, "the radio will not serve our identity, so peers cannot name us");
        }
    }
}
