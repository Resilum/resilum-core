//! stdin/stdout ↔ runner glue for PipeInterface subprocess mode. Used only
//! where the runner is a subprocess; an embedder bridges it in-process instead.

use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::sync::mpsc::{Sender, channel};
use std::thread;

use leviculum_std::api::Identity;
use resilum_core::covert::carrier::{CarrierClient, CarrierServer};
use resilum_core::covert::runner;

const STDIN_CHUNK: usize = 4096;

pub fn client<C>(carrier: C, server: Identity, session_id: u32) -> io::Result<()>
where
    C: CarrierClient + Send + Sync + 'static,
{
    let (tx, rx) = channel();
    spawn_stdin_reader(tx);
    runner::run_client(carrier, server, session_id, rx, write_stdout)
}

pub fn server<S>(carrier: S, identity: Identity) -> io::Result<()>
where
    S: CarrierServer<ReplyTo = IpAddr> + Send + Sync + 'static,
{
    let (tx, rx) = channel();
    spawn_stdin_reader(tx);
    runner::run_server(carrier, identity, rx, |_sid, data| write_stdout(data))
}

fn spawn_stdin_reader(tx: Sender<Vec<u8>>) {
    thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut buf = [0u8; STDIN_CHUNK];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        return;
                    }
                }
            }
        }
    });
}

fn write_stdout(data: &[u8]) {
    let mut out = io::stdout().lock();
    let _ = out.write_all(data);
    let _ = out.flush();
}
