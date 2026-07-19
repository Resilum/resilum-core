/// Events emitted by a node, drained by the daemon and the FFI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The node started.
    Started,
    /// The node stopped.
    Stopped,
    /// A peer/anchor became reachable (destination hash bytes).
    PeerDiscovered(Vec<u8>),
    /// Inbound data arrived.
    Received {
        /// Source destination hash.
        source: Vec<u8>,
        /// Payload bytes.
        data: Vec<u8>,
    },
}
