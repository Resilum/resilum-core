/// Events emitted by a node, drained by the daemon and the FFI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started,
    Stopped,
    /// A peer/anchor became reachable (destination hash).
    PeerDiscovered(Vec<u8>),
    /// Inbound data arrived.
    Received {
        source: Vec<u8>,
        data: Vec<u8>,
    },
}
