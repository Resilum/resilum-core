mod carry;

use std::sync::Arc;

use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::radio::{ConnectionId, Radio, Role};
use super::spec;

const PIPE_BUFFER_EQUIVALENT: usize = 64 * 1024;
const FRAGMENTS_HELD_PER_PEER: usize = 64;

pub struct PeerLink {
    pub peer: [u8; spec::IDENTITY_LEN],
    pub conn: ConnectionId,
    pub interface: leviculum_std::InterfaceId,
    arriving: mpsc::Sender<Vec<u8>>,
    pumps: [JoinHandle<()>; 2],
    _detaches_when_dropped: ByteChannelHandle,
}

impl PeerLink {
    pub fn open(
        engine: &Arc<ReticulumNode>,
        name: &str,
        radio: Arc<dyn Radio>,
        conn: ConnectionId,
        role: Role,
        peer: [u8; spec::IDENTITY_LEN],
        now_ms: impl Fn() -> u64 + Send + 'static,
    ) -> Result<Self, String> {
        let (lev_side, our_side) = tokio::io::duplex(PIPE_BUFFER_EQUIVALENT);
        let handle = engine
            .spawn_byte_channel(name, lev_side)
            .map_err(|e| format!("attach byte channel: {e}"))?;
        let (ours_read, ours_write) = tokio::io::split(our_side);
        let carried = radio.bytes_one_write_carries(conn);
        let (arriving, arrived) = mpsc::channel(FRAGMENTS_HELD_PER_PEER);
        Ok(Self {
            peer,
            conn,
            interface: handle.id(),
            arriving,
            pumps: [
                tokio::spawn(carry::out_to_the_radio(ours_read, radio, conn, role)),
                tokio::spawn(carry::in_from_the_radio(
                    ours_write, arrived, carried, now_ms,
                )),
            ],
            _detaches_when_dropped: handle,
        })
    }

    pub fn hand_over(&self, fragment: Vec<u8>) {
        let _ = self.arriving.try_send(fragment);
    }
}

impl Drop for PeerLink {
    fn drop(&mut self) {
        for pump in &self.pumps {
            pump.abort();
        }
    }
}
