//! Carrier contract. The engine passes opaque `wire` bytes and gets them back;
//! `reply_to` only addresses the answer — demux is by the datagram's session,
//! so a carrier stays oblivious to seq/ack/auth.
//!
//! Implementations planned:
//! - `icmp` — ICMP echo (raw AF_INET/AF_PACKET, Linux `CAP_NET_RAW`).
//! - `dns`, `arp`, `ntp`, `dhcp`, `snmp` — future.

/// Address the engine hands back to the carrier to answer a specific request.
/// Carriers pick their own concrete type behind `Any` (an IPv4 for ICMP, a
/// MAC/interface pair for ARP, etc.) so the engine stays carrier-agnostic.
pub type ReplyTo = Box<dyn std::any::Any + Send + Sync>;

/// Whether the carrier drives payload delivery from the client (poll) or the
/// server can push replies asynchronously (push). ICMP is `Poll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Semantics {
    Poll,
    Push,
}

/// Fully-built carrier frame ready for the wire (raw bytes plus whatever
/// metadata the carrier needs to send them; carriers own the shape).
pub type Packet = Box<dyn std::any::Any + Send + Sync>;

/// Bytes lifted off the wire, still carrier-shaped. The engine hands each raw
/// back to the carrier's `parse_*` for the payload it needs.
pub type Raw = Box<dyn std::any::Any + Send + Sync>;

/// A covert transport carrier. Implementations move `wire` bytes through a
/// medium (ICMP echo, DNS query, ARP request, …) with as little visible
/// singularity as possible.
pub trait Carrier: Send + Sync {
    /// A short name, used in logs and configuration (`"icmp"`, `"dns"`, …).
    fn name(&self) -> &str;

    /// Poll vs push. See [`Semantics`].
    fn semantics(&self) -> Semantics {
        Semantics::Poll
    }

    /// Auth-tag length the engine reserves per datagram (default 8 bytes).
    fn tag_len(&self) -> usize {
        8
    }

    /// Max payload the engine may put in one packet, before framing overhead.
    fn capacity(&self) -> usize;

    /// Capacity when answering `reply_to` — override for carriers whose budget
    /// depends on the peer (IPv4 vs IPv6 headers, DNS label length, …).
    fn capacity_for(&self, _reply_to: &ReplyTo) -> usize {
        self.capacity()
    }

    /// Wrap outbound `wire` into a client-side request packet. `reply_to` is
    /// `None` on the client (its own address is the source anyway).
    fn build_request(&self, reply_to: Option<&ReplyTo>, wire: &[u8]) -> Packet;

    /// Peel an inbound request; returns `(reply_to, wire)` when the packet is
    /// one of ours, `None` when it should be ignored.
    fn parse_request(&self, raw: &Raw) -> Option<(ReplyTo, Vec<u8>)>;

    /// Wrap outbound `wire` into a server-side response packet addressed to
    /// `reply_to` (the client the request came from).
    fn build_response(&self, reply_to: &ReplyTo, wire: &[u8]) -> Packet;

    /// Peel an inbound response; returns just the payload bytes.
    fn parse_response(&self, raw: &Raw) -> Option<Vec<u8>>;

    /// Send `packet` on the wire. Errors are logged inside the carrier; the
    /// engine has no way to recover per-packet failures.
    fn send(&self, packet: Packet);
}
