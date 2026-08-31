mod obey;
mod peers;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blew::central::Central;
use blew::peripheral::Peripheral;
use tokio::sync::mpsc;

use super::command::Command;
use crate::ble::radio::{ConnectionId, Outbound, RadioError, RadioEvent};
use peers::Peers;

const ATT_HEADER: usize = 3;

pub(super) async fn spawn(
    commands: mpsc::Receiver<Command>,
    outbound: mpsc::Receiver<Outbound>,
    telling: mpsc::Sender<RadioEvent>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
) -> Result<(), RadioError> {
    let central = Central::new().await.map_err(cannot)?;
    let peripheral = Peripheral::new().await.map_err(cannot)?;
    tokio::spawn(run(Held {
        central,
        peripheral,
        commands,
        outbound,
        telling,
        carried,
        peers: Peers::default(),
    }));
    Ok(())
}

struct Held {
    central: Central,
    peripheral: Peripheral,
    commands: mpsc::Receiver<Command>,
    outbound: mpsc::Receiver<Outbound>,
    telling: mpsc::Sender<RadioEvent>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
    peers: Peers,
}

async fn run(mut held: Held) {
    loop {
        tokio::select! {
            asked = held.commands.recv() => match asked {
                None => return,
                Some(command) => obey::command(&mut held, command).await,
            },
            piece = held.outbound.recv() => match piece {
                None => return,
                Some(piece) => obey::put_on_the_air(&held, piece).await,
            },
        }
    }
}

fn cannot(why: impl std::fmt::Display) -> RadioError {
    RadioError::Backend(why.to_string())
}
