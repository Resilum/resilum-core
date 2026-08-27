//! In-process covert attach: bridge the covert runner to leviculum through an
//! in-memory byte channel, replacing the subprocess-behind-a-PipeInterface path
//! (for embedders that run no separate process). Carrier-agnostic — per-carrier
//! construction lives in the submodules.

mod icmp;

use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::DialableAddress;

pub(in crate::discovery::covert) fn attach(
    engine: &Arc<ReticulumNode>,
    name: &str,
    carrier: &str,
    addr: &DialableAddress,
    server_pubkey: &[u8],
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    match carrier {
        "icmp" => icmp::attach(engine, name, addr, server_pubkey, mtu),
        other => Err(format!("covert carrier not supported in-process: {other}")),
    }
}

pub fn listen(
    engine: &Arc<ReticulumNode>,
    name: &str,
    carrier: &str,
    identity: Identity,
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    match carrier {
        "icmp" => icmp::listen(engine, name, identity, mtu),
        other => Err(format!("covert carrier not supported in-process: {other}")),
    }
}

const PIPE_BUFFER_EQUIVALENT: usize = 64 * 1024;
const PUMP_CHUNK: usize = 4096;

type Duplex = tokio::io::DuplexStream;

pub(in crate::discovery::covert) struct Decoded(UnboundedSender<Vec<u8>>);

impl Decoded {
    pub(in crate::discovery::covert) fn hand_to_leviculum(&self, bytes: &[u8]) {
        let _ = self.0.send(bytes.to_vec());
    }
}

fn bridge<R>(engine: &Arc<ReticulumNode>, name: &str, run: R) -> Result<ByteChannelHandle, String>
where
    R: FnOnce(std::sync::mpsc::Receiver<Vec<u8>>, Decoded) + Send + 'static,
{
    let (lev_side, our_side) = tokio::io::duplex(PIPE_BUFFER_EQUIVALENT);
    let iface = engine
        .spawn_byte_channel(name, lev_side)
        .map_err(|e| format!("attach byte channel: {e}"))?;
    let (our_read, our_write) = tokio::io::split(our_side);

    let (uplink_tx, uplink_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    tokio::spawn(carry_out_what_leviculum_writes(our_read, uplink_tx));

    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(give_leviculum_what_arrived(our_write, out_rx));

    std::thread::spawn(move || run(uplink_rx, Decoded(out_tx)));

    Ok(iface)
}

async fn carry_out_what_leviculum_writes(
    mut source: ReadHalf<Duplex>,
    uplink: std::sync::mpsc::Sender<Vec<u8>>,
) {
    let mut buf = [0u8; PUMP_CHUNK];
    loop {
        match source.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                if uplink.send(buf[..n].to_vec()).is_err() {
                    return;
                }
            }
        }
    }
}

async fn give_leviculum_what_arrived(
    mut sink: WriteHalf<Duplex>,
    mut decoded: UnboundedReceiver<Vec<u8>>,
) {
    while let Some(bytes) = decoded.recv().await {
        if sink.write_all(&bytes).await.is_err() {
            return;
        }
        let _ = sink.flush().await;
    }
}

fn random_session_id() -> u32 {
    use rand_core::RngCore;
    let mut buf = [0u8; 4];
    rand_core::OsRng.fill_bytes(&mut buf);
    u32::from_be_bytes(buf)
}
