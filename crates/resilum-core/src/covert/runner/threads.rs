//! Fan-in helpers: caller's uplink channel and carrier packets funnel into
//! the runner's internal `Msg` channel.

use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use super::drive::Msg;
use crate::covert::carrier::{CarrierClient, CarrierServer};

const CARRIER_BUF: usize = 65535;

pub fn spawn_uplink(uplink: Receiver<Vec<u8>>, out: Sender<Msg>) {
    thread::spawn(move || {
        while let Ok(chunk) = uplink.recv() {
            if out.send(Msg::Tx(chunk)).is_err() {
                return;
            }
        }
        let _ = out.send(Msg::Eof);
    });
}

pub fn spawn_client_sniffer<C>(carrier: Arc<C>, out: Sender<Msg>)
where
    C: CarrierClient + Send + Sync + 'static,
{
    thread::spawn(move || {
        let mut buf = vec![0u8; CARRIER_BUF];
        loop {
            match carrier.recv_response(&mut buf) {
                Ok(Some(payload)) => {
                    if out.send(Msg::Rx(payload)).is_err() {
                        return;
                    }
                }
                Ok(None) if carrier.told_to_stop() => return,
                Ok(None) => continue,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    tracing::error!(error = %e, "carrier recv failed");
                    return;
                }
            }
        }
    });
}

pub fn spawn_server_sniffer<S>(carrier: Arc<S>, out: Sender<Msg>)
where
    S: CarrierServer<ReplyTo = IpAddr> + Send + Sync + 'static,
{
    thread::spawn(move || {
        let mut buf = vec![0u8; CARRIER_BUF];
        loop {
            match carrier.recv_request(&mut buf) {
                Ok(Some((peer, payload))) => {
                    if out.send(Msg::RxServer(peer, payload)).is_err() {
                        return;
                    }
                }
                Ok(None) if carrier.told_to_stop() => return,
                Ok(None) => continue,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    tracing::error!(error = %e, "carrier recv_request failed");
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests;
