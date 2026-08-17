//! Transport-agnostic pump: bytes flow between an `uplink` channel and the
//! engine, engine output goes to an `on_output` closure. Callers plug either
//! a PipeInterface subprocess (stdin/stdout) or an in-process bridge into
//! these two seams — the runner itself does no I/O.

mod drive;
mod threads;

use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use leviculum_std::api::Identity;

use super::carrier::{CarrierClient, CarrierServer};
use super::engine::client::ClientEngine;
use super::engine::server::ServerEngine;

const MIN_POLL: Duration = Duration::from_millis(200);
const MAX_POLL: Duration = Duration::from_secs(30);

pub fn run_client<C, F>(
    carrier: C,
    server: Identity,
    session_id: u32,
    uplink: Receiver<Vec<u8>>,
    on_output: F,
) -> io::Result<()>
where
    C: CarrierClient + Send + Sync + 'static,
    F: FnMut(&[u8]) + Send + 'static,
{
    let carrier = Arc::new(carrier);
    let (tx, rx) = channel();
    threads::spawn_uplink(uplink, tx.clone());
    threads::spawn_client_sniffer(Arc::clone(&carrier), tx);

    let mut engine = ClientEngine::new(
        Arc::clone(&carrier),
        server,
        session_id,
        Box::new(on_output),
        MIN_POLL,
        MAX_POLL,
    );
    drive::run(rx, |ev, now| match ev {
        drive::Event::Msg(drive::Msg::Rx(raw)) => engine.on_received(&raw, now),
        drive::Event::Msg(drive::Msg::Tx(chunk)) => {
            engine.write(&chunk);
            Ok(())
        }
        drive::Event::Tick => engine.poll(now),
        _ => Ok(()),
    })
}

pub fn run_server<S, F>(
    carrier: S,
    identity: Identity,
    uplink: Receiver<Vec<u8>>,
    on_output: F,
) -> io::Result<()>
where
    S: CarrierServer<ReplyTo = IpAddr> + Send + Sync + 'static,
    F: FnMut(u32, &[u8]) + Send + 'static,
{
    let carrier = Arc::new(carrier);
    let (tx, rx) = channel();
    threads::spawn_uplink(uplink, tx.clone());
    threads::spawn_server_sniffer(Arc::clone(&carrier), tx);

    let mut engine = ServerEngine::new(carrier, identity, Box::new(on_output), None);
    drive::run(rx, |ev, now| match ev {
        drive::Event::Msg(drive::Msg::RxServer(peer, raw)) => engine.on_received(peer, &raw, now),
        drive::Event::Msg(drive::Msg::Tx(chunk)) => {
            engine.broadcast(&chunk);
            Ok(())
        }
        drive::Event::Tick => {
            engine.poll(now);
            Ok(())
        }
        _ => Ok(()),
    })
}
