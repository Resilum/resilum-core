mod as_a_central;
mod as_a_peripheral;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blew::DeviceId;
use tokio::sync::mpsc;

use super::Reporting;
use crate::ble::backend::drive::peers::Peers;
use crate::ble::radio::{ConnectionId, PeerAddress, RadioEvent};

struct Bench {
    told: Reporting,
    heard: mpsc::Receiver<RadioEvent>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
}

fn bench() -> Bench {
    let (telling, heard) = mpsc::channel(16);
    let carried = Arc::new(Mutex::new(HashMap::new()));
    Bench {
        told: Reporting {
            telling,
            carried: Arc::clone(&carried),
            peers: Peers::default(),
        },
        heard,
        carried,
    }
}

fn them() -> DeviceId {
    DeviceId::from("aa:bb:cc:dd:ee:ff")
}

fn their_address() -> PeerAddress {
    PeerAddress("aa:bb:cc:dd:ee:ff".to_owned())
}
