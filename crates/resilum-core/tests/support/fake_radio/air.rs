use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::mpsc;

use resilum_core::ble::radio::{ConnectionId, PeerAddress, RadioEvent, Role};
use resilum_core::ble::spec;

pub struct Endpoint {
    pub address: PeerAddress,
    pub role: Role,
    pub to: mpsc::Sender<RadioEvent>,
}

#[derive(Default)]
pub struct Wire {
    listening: HashMap<PeerAddress, mpsc::Sender<RadioEvent>>,
    announced: HashMap<PeerAddress, (String, Vec<u8>)>,
    identities: HashMap<PeerAddress, [u8; spec::IDENTITY_LEN]>,
    ends: HashMap<ConnectionId, [Endpoint; 2]>,
    next_conn: u64,
    pub carried_per_write: usize,
    pub tapped: Vec<(ConnectionId, u128, Vec<u8>)>,
}

#[derive(Clone, Default)]
pub struct Air(Arc<Mutex<Wire>>);

impl Air {
    #[must_use]
    pub fn new(carried_per_write: usize) -> Self {
        let air = Self::default();
        air.wire().carried_per_write = carried_per_write;
        air
    }

    pub fn wire(&self) -> MutexGuard<'_, Wire> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn advertise(
        &self,
        at: &PeerAddress,
        name: &str,
        beacon: &[u8],
        to: mpsc::Sender<RadioEvent>,
    ) {
        let mut wire = self.wire();
        wire.announced
            .insert(at.clone(), (name.to_owned(), beacon.to_vec()));
        wire.listening.insert(at.clone(), to);
        let arriving = RadioEvent::Seen {
            address: at.clone(),
            name: Some(name.to_owned()),
            beacon: beacon.to_vec(),
        };
        for (address, listener) in &wire.listening {
            if address != at {
                let _ = listener.try_send(arriving.clone());
            }
        }
    }

    pub fn seen_by(&self, scanner: &PeerAddress) -> Vec<RadioEvent> {
        self.wire()
            .announced
            .iter()
            .filter(|(address, _)| *address != scanner)
            .map(|(address, (name, beacon))| RadioEvent::Seen {
                address: address.clone(),
                name: Some(name.clone()),
                beacon: beacon.clone(),
            })
            .collect()
    }

    pub fn serve_identity(&self, at: &PeerAddress, identity: [u8; spec::IDENTITY_LEN]) {
        self.wire().identities.insert(at.clone(), identity);
    }

    pub fn identity_of(&self, address: &PeerAddress) -> Option<[u8; spec::IDENTITY_LEN]> {
        self.wire().identities.get(address).copied()
    }

    pub fn join(
        &self,
        dialer: &PeerAddress,
        dialer_to: mpsc::Sender<RadioEvent>,
        answerer: &PeerAddress,
    ) -> Option<ConnectionId> {
        let mut wire = self.wire();
        let answerer_to = wire.listening.get(answerer)?.clone();
        wire.next_conn += 1;
        let conn = ConnectionId(wire.next_conn);
        let carried = wire.carried_per_write;
        wire.ends.insert(
            conn,
            [
                Endpoint {
                    address: dialer.clone(),
                    role: Role::Central,
                    to: dialer_to,
                },
                Endpoint {
                    address: answerer.clone(),
                    role: Role::Peripheral,
                    to: answerer_to,
                },
            ],
        );
        let ends = wire.ends.get(&conn)?;
        for (nth, end) in ends.iter().enumerate() {
            let _ = end.to.try_send(RadioEvent::Connected {
                conn,
                address: ends[1 - nth].address.clone(),
                role: end.role,
                bytes_one_write_carries: carried,
            });
        }
        Some(conn)
    }

    pub fn far_end(&self, conn: ConnectionId, ours: &PeerAddress) -> Option<Endpoint> {
        let wire = self.wire();
        let ends = wire.ends.get(&conn)?;
        ends.iter()
            .find(|end| end.address != *ours)
            .map(|end| Endpoint {
                address: end.address.clone(),
                role: end.role,
                to: end.to.clone(),
            })
    }

    pub fn part(&self, conn: ConnectionId) {
        let mut wire = self.wire();
        if let Some(ends) = wire.ends.remove(&conn) {
            for end in ends {
                let _ = end.to.try_send(RadioEvent::Disconnected { conn });
            }
        }
    }
}
