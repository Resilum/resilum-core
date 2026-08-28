//! Portable ICMP client: `SOCK_DGRAM + IPPROTO_ICMP{,V6}`. No root, no raw.
//! Works on Linux (subject to `net.ipv4.ping_group_range`), Android with a
//! plain `INTERNET` permission and iOS without entitlements.
//!
//! The kernel writes the IP header on send and strips it on recv, and it
//! matches replies to our socket by its bound identifier — foreign echoes
//! from other pings never reach us.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;
use std::time::Duration;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::wake::{Ready, Wake};
use super::wire;
use crate::covert::carrier::CarrierClient;

/// Default MTU used when the caller supplies none. The tunnel operator can
/// override per-carrier from the covert config (see `spec::covert::CovertSpec`).
pub const DEFAULT_MTU: usize = 1400;
const IPV4_OVERHEAD: usize = 20 + 8;
const IPV6_OVERHEAD: usize = 40 + 8;

pub struct IcmpClient {
    server: IpAddr,
    marker: [u8; super::marker::MARKER_LEN],
    mtu: usize,
    sock: Socket,
    wake: Wake,
}

impl IcmpClient {
    /// Open the ICMP socket for `server` with the default MTU.
    pub fn new(server: IpAddr, marker: [u8; super::marker::MARKER_LEN]) -> io::Result<Self> {
        Self::with_mtu(server, marker, DEFAULT_MTU)
    }

    /// `marker` is the per-server tunnel marker both peers derive from the
    /// server's public key (see [`super::marker::tunnel_marker`]).
    pub fn with_mtu(
        server: IpAddr,
        marker: [u8; super::marker::MARKER_LEN],
        mtu: usize,
    ) -> io::Result<Self> {
        let (domain, proto) = match server {
            IpAddr::V4(_) => (Domain::IPV4, Protocol::ICMPV4),
            IpAddr::V6(_) => (Domain::IPV6, Protocol::ICMPV6),
        };
        let sock = Socket::new(domain, Type::DGRAM, Some(proto))?;
        sock.set_nonblocking(false)?;
        Ok(Self {
            server,
            marker,
            mtu,
            sock,
            wake: Wake::new()?,
        })
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.sock.set_read_timeout(timeout)
    }

    /// Wrap `payload` in an echo-request and send it to the server.
    pub fn send(&self, payload: &[u8]) -> io::Result<()> {
        let body = wire::build_echo_request(self.marker, payload, self.server.is_ipv6());
        let addr: SockAddr = SocketAddr::new(self.server, 0).into();
        self.sock.send_to(&body, &addr).map(|_| ())
    }

    /// Block until the next reply arrives, returning its payload when it
    /// carries our tunnel id (foreign echoes are silently skipped).
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<Option<Vec<u8>>> {
        if let Ready::Woken = self
            .wake
            .wait_for_carrier_or_a_raise(&[self.sock.as_raw_fd()])?
        {
            return Ok(None);
        }
        let cell = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), buf.len()) };
        let (n, _addr) = self.sock.recv_from(cell)?;
        let body = &buf[..n];
        let v6 = self.server.is_ipv6();
        Ok(wire::payload_of_reply(body, self.marker, v6).map(<[u8]>::to_vec))
    }
}

impl CarrierClient for IcmpClient {
    fn capacity(&self) -> usize {
        self.mtu
            - if self.server.is_ipv6() {
                IPV6_OVERHEAD
            } else {
                IPV4_OVERHEAD
            }
    }
    fn send_request(&self, wire: &[u8]) -> io::Result<()> {
        self.send(wire)
    }
    fn recv_response(&self, buf: &mut [u8]) -> io::Result<Option<Vec<u8>>> {
        self.recv(buf)
    }
    fn stop_receiving(&self) {
        self.wake.raise();
    }
    fn told_to_stop(&self) -> bool {
        self.wake.raised()
    }
}
