use std::sync::Arc;

use leviculum_core::framing::hdlc::{DeframeResult, Deframer, frame};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::mpsc;

use crate::ble::framing::{Arrived, Reassembly, fragment};
use crate::ble::radio::{ConnectionId, Outbound, Radio, Role};
use crate::ble::spec;

type Duplex = tokio::io::DuplexStream;

const READ_CHUNK: usize = 1024;

pub(super) async fn out_to_the_radio(
    mut ours: ReadHalf<Duplex>,
    radio: Arc<dyn Radio>,
    conn: ConnectionId,
    role: Role,
) {
    let mut deframer = Deframer::new();
    let mut buf = [0u8; READ_CHUNK];
    let outbound = radio.outbound_awaits_a_slot();
    loop {
        let read = match ours.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        for found in deframer.process(&buf[..read]) {
            let DeframeResult::Frame(packet) = found else {
                continue;
            };
            let carried = radio.bytes_one_write_carries(conn);
            for piece in fragment(&packet, carried) {
                let waiting = Outbound {
                    conn,
                    characteristic: role.sends_on(),
                    value: piece,
                    acknowledged: false,
                };
                if outbound.send(waiting).await.is_err() {
                    return;
                }
            }
        }
        if deframer.buffer_len() > spec::LARGEST_PACKET {
            deframer.reset();
        }
    }
}

pub(super) async fn in_from_the_radio(
    mut ours: WriteHalf<Duplex>,
    mut arriving: mpsc::Receiver<Vec<u8>>,
    carried: usize,
    now_ms: impl Fn() -> u64 + Send,
) {
    let mut held = Reassembly::default();
    let mut framed = Vec::new();
    while let Some(piece) = arriving.recv().await {
        let Arrived::Packet(packet) = held.process(&piece, carried, now_ms()) else {
            continue;
        };
        if packet == spec::KEEPALIVE_PACKET {
            continue;
        }
        framed.clear();
        frame(&packet, &mut framed);
        if ours.write_all(&framed).await.is_err() || ours.flush().await.is_err() {
            return;
        }
    }
}
