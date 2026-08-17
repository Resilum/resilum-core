//! In-process covert attach: bridge the covert runner to leviculum through an
//! in-memory byte channel, replacing the subprocess-behind-a-PipeInterface path
//! (for embedders that run no separate process). Carrier-agnostic — per-carrier
//! construction lives in the submodules.

mod icmp;

use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::covert::carrier::CarrierClient;
use crate::covert::runner;

/// Attach a covert client for `carrier` to `addr` in-process. Errors on a
/// carrier not supported here.
pub(in crate::discovery::covert) fn attach(
    engine: &Arc<ReticulumNode>,
    name: &str,
    carrier: &str,
    addr: &str,
    server_pubkey: &[u8],
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    match carrier {
        "icmp" => icmp::attach(engine, name, addr, server_pubkey, mtu),
        other => Err(format!("covert carrier not supported in-process: {other}")),
    }
}

/// Matches the Linux default pipe buffer the subprocess PipeInterface used, so
/// the in-memory duplex has equivalent backpressure headroom.
const DUPLEX_BUF: usize = 64 * 1024;
/// Per-read grab size draining the duplex into the uplink (resilumd's stdin
/// chunk). Covert is byte-transparent, so chunk boundaries don't matter.
const PUMP_CHUNK: usize = 4096;

/// Bridge an already-built `carrier` to leviculum in-process, returning the
/// interface handle (drop to detach).
///
/// Must be called within the node's tokio runtime (it spawns the bridge tasks
/// there). The blocking covert runner gets its own OS thread and exits when the
/// duplex closes.
fn bridge<C>(
    engine: &Arc<ReticulumNode>,
    name: &str,
    carrier: C,
    server: Identity,
    session_id: u32,
) -> Result<ByteChannelHandle, String>
where
    C: CarrierClient + Send + Sync + 'static,
{
    let (lev_side, our_side) = tokio::io::duplex(DUPLEX_BUF);
    let iface = engine
        .spawn_byte_channel(name, lev_side)
        .map_err(|e| format!("attach byte channel: {e}"))?;
    let (mut our_read, mut our_write) = tokio::io::split(our_side);

    // leviculum → uplink: framed bytes leviculum emits are carried out covertly.
    let (uplink_tx, uplink_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    tokio::spawn(async move {
        let mut buf = [0u8; PUMP_CHUNK];
        loop {
            match our_read.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if uplink_tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // covert → leviculum: decoded bytes go back onto the duplex.
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if our_write.write_all(&bytes).await.is_err() {
                break;
            }
            let _ = our_write.flush().await;
        }
    });

    std::thread::spawn(move || {
        let _ = runner::run_client(carrier, server, session_id, uplink_rx, move |bytes| {
            let _ = out_tx.send(bytes.to_vec());
        });
    });

    Ok(iface)
}

fn random_session_id() -> u32 {
    use rand_core::RngCore;
    let mut buf = [0u8; 4];
    rand_core::OsRng.fill_bytes(&mut buf);
    u32::from_be_bytes(buf)
}
