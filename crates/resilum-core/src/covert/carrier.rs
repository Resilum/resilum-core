//! Carrier contracts used by the covert engine. Two halves so a build without
//! server capability (mobile client) can implement only the client one.
//!
//! `ReplyTo` is the carrier-shaped return address (an `IpAddr` for ICMP, a
//! `(SocketAddr, DomainName)` for DNS, a MAC for ARP, etc). The engine is
//! opaque to its shape.

use std::io;

pub trait CarrierClient {
    fn tag_len(&self) -> usize {
        8
    }
    /// Payload budget the engine may put in one outbound datagram, minus tag
    /// and header. Engine reserves those.
    fn capacity(&self) -> usize;
    /// Wrap `wire` in a request packet and send it upstream.
    fn send_request(&self, wire: &[u8]) -> io::Result<()>;
    /// Read the next inbound response payload, or `Ok(None)` on a foreign
    /// frame the carrier filtered out.
    fn recv_response(&self, buf: &mut [u8]) -> io::Result<Option<Vec<u8>>>;
}

pub trait CarrierServer {
    type ReplyTo: Clone + Send + Sync + 'static;
    fn tag_len(&self) -> usize {
        8
    }
    fn capacity_for(&self, reply_to: &Self::ReplyTo) -> usize;
    fn send_response(&self, reply_to: &Self::ReplyTo, wire: &[u8]) -> io::Result<()>;
    fn recv_request(&self, buf: &mut [u8]) -> io::Result<Option<(Self::ReplyTo, Vec<u8>)>>;
}
