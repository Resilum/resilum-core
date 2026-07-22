//! Main event loop: dequeue carrier/stdin messages, dispatch, then tick.

use std::io;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

const POLL_TICK: Duration = Duration::from_millis(100);

pub enum Msg {
    Rx(Vec<u8>),
    RxServer(IpAddr, Vec<u8>),
    Tx(Vec<u8>),
    Eof,
}

pub enum Event {
    Msg(Msg),
    Tick,
}

pub fn run<F>(rx: Receiver<Msg>, mut step: F) -> io::Result<()>
where
    F: FnMut(Event, f64) -> io::Result<()>,
{
    let start = Instant::now();
    loop {
        let now = start.elapsed().as_secs_f64();
        match rx.recv_timeout(POLL_TICK) {
            Ok(Msg::Eof) => return Ok(()),
            Ok(msg) => step(Event::Msg(msg), now)?,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        loop {
            match rx.try_recv() {
                Ok(Msg::Eof) => return Ok(()),
                Ok(msg) => step(Event::Msg(msg), start.elapsed().as_secs_f64())?,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        step(Event::Tick, start.elapsed().as_secs_f64())?;
    }
}
