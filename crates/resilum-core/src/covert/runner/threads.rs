//! Fan-in helpers: caller's uplink channel and carrier packets funnel into
//! the runner's internal `Msg` channel.

use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use super::drive::Msg;
use crate::covert::carrier::{CarrierClient, CarrierServer};

const CARRIER_BUF: usize = 65535;

pub fn spawn_uplink(uplink: Receiver<Vec<u8>>, out: Sender<Msg>) {
    let started = resilum_tasks::a_thread_of_its_own("covert: what we send out", move || {
        while let Ok(chunk) = uplink.recv() {
            if out.send(Msg::Tx(chunk)).is_err() {
                return;
            }
        }
        if out.send(Msg::Eof).is_err() {
            tracing::debug!("the covert runner had already gone when the uplink ended");
        }
    });
    or_say_the_carrier_is_deaf(started, "what we send out");
}

fn or_say_the_carrier_is_deaf(started: std::io::Result<JoinHandle<()>>, what: &str) {
    if let Err(e) = started {
        tracing::error!(error = %e, %what, "no thread for the covert carrier");
    }
}

pub fn spawn_client_sniffer<C>(carrier: Arc<C>, out: Sender<Msg>)
where
    C: CarrierClient + Send + Sync + 'static,
{
    let started = resilum_tasks::a_thread_of_its_own("covert: replies coming back", move || {
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
    or_say_the_carrier_is_deaf(started, "replies coming back");
}

pub fn spawn_server_sniffer<S>(carrier: Arc<S>, out: Sender<Msg>)
where
    S: CarrierServer<ReplyTo = IpAddr> + Send + Sync + 'static,
{
    let started = resilum_tasks::a_thread_of_its_own("covert: requests arriving", move || {
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
    or_say_the_carrier_is_deaf(started, "requests arriving");
}

#[cfg(test)]
mod tests;
