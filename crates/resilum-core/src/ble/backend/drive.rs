mod dialling;
mod listen;
mod obey;
mod peers;
mod sending;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use blew::central::{Central, CentralEvent};
use blew::peripheral::{Peripheral, PeripheralRequest, PeripheralStateEvent};
use futures::StreamExt as _;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::command::Command;
use crate::ble::radio::{ConnectionId, Outbound, PeerAddress, RadioError, RadioEvent};
use dialling::Dialled;
use peers::Peers;

const ATT_HEADER: usize = 3;

type Arriving<T> = Box<dyn futures::Stream<Item = T> + Unpin + Send + Sync>;

pub(super) async fn spawn(
    commands: mpsc::Receiver<Command>,
    outbound: mpsc::Receiver<Outbound>,
    telling: mpsc::Sender<RadioEvent>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
) -> Result<(), RadioError> {
    let central = Central::new().await.map_err(cannot)?;
    let peripheral = Peripheral::new().await.map_err(cannot)?;
    let heard = Box::new(central.events());
    let watched = Box::new(peripheral.state_events());
    let asked = peripheral
        .take_requests()
        .ok_or_else(|| RadioError::Backend("the radio hands its requests out once".into()))?;
    tokio::spawn(run(Held {
        central: Arc::new(central),
        peripheral,
        commands,
        outbound,
        told: Reporting {
            telling,
            carried,
            peers: Peers::default(),
        },
        heard,
        watched,
        asked: Box::new(asked),
        already_dialling: HashSet::new(),
        dials_in_flight: JoinSet::new(),
    }));
    Ok(())
}

pub(super) struct Reporting {
    telling: mpsc::Sender<RadioEvent>,
    carried: Arc<Mutex<HashMap<ConnectionId, usize>>>,
    peers: Peers,
}

impl Reporting {
    fn now_carries(&self, conn: ConnectionId, bytes: usize) {
        self.carried
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(conn, bytes);
    }

    fn forget_what_it_carried(&self, conn: ConnectionId) {
        self.carried
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&conn);
    }
}

struct Held {
    central: Arc<Central>,
    peripheral: Peripheral,
    commands: mpsc::Receiver<Command>,
    outbound: mpsc::Receiver<Outbound>,
    told: Reporting,
    heard: Arriving<CentralEvent>,
    watched: Arriving<PeripheralStateEvent>,
    asked: Arriving<PeripheralRequest>,
    already_dialling: HashSet<PeerAddress>,
    dials_in_flight: JoinSet<Dialled>,
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
                Some(piece) => sending::put_on_the_air(&held, piece).await,
            },
            event = held.heard.next() => match event {
                None => held.heard = nothing_more(),
                Some(event) => listen::what_the_central_heard(&mut held.told, event).await,
            },
            event = held.watched.next() => match event {
                None => held.watched = nothing_more(),
                Some(event) => listen::what_the_peripheral_saw(&mut held.told, event).await,
            },
            reached = held.dials_in_flight.join_next(), if !held.dials_in_flight.is_empty() => match reached {
                Some(Ok(reached)) => dialling::dialled(&mut held, reached).await,
                Some(Err(error)) => tracing::error!(%error, "a dial died on the way"),
                None => {}
            },
            request = held.asked.next() => match request {
                None => held.asked = nothing_more(),
                Some(request) => listen::what_a_central_asked(&mut held.told, request).await,
            },
        }
    }
}

fn nothing_more<T: Send + Sync + 'static>() -> Arriving<T> {
    Box::new(futures::stream::pending())
}

fn cannot(why: impl std::fmt::Display) -> RadioError {
    RadioError::Backend(why.to_string())
}
