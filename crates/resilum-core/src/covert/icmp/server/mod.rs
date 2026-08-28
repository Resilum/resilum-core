//! Covert ICMP server side. Linux only.
//!
//! Send: `SOCK_RAW + IPPROTO_ICMP{V4,V6}` — the kernel adds the IP header.
//! Sniff: `AF_PACKET` + BPF ICMP filter. Callers must first install
//! `super::nftguard::Guard` so the kernel does not answer our marked
//! echo-requests itself.

#![cfg(target_os = "linux")]

mod sniff;

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;
use std::sync::Mutex;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::marker::MARKER_LEN;
use super::wake::{Ready, Wake};
use super::wire;
use crate::covert::carrier::CarrierServer;

pub use super::client::DEFAULT_MTU;
const IPV4_OVERHEAD: usize = 20 + 8;
const IPV6_OVERHEAD: usize = 40 + 8;

pub struct IcmpServer {
    marker: [u8; MARKER_LEN],
    mtu: usize,
    send4: Socket,
    send6: Option<Socket>,
    sniff4: Socket,
    sniff6: Option<Socket>,
    reply_id: Mutex<HashMap<IpAddr, u16>>,
    _kernel_stays_quiet: super::nftguard::Guard,
    wake: Wake,
}

impl IcmpServer {
    pub fn new(marker: [u8; MARKER_LEN]) -> io::Result<Self> {
        Self::with_mtu(marker, DEFAULT_MTU)
    }

    pub fn with_mtu(marker: [u8; MARKER_LEN], mtu: usize) -> io::Result<Self> {
        let send4 = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
        let send6 = Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6)).ok();
        let sniff4 = sniff::sniff_socket(wire::ETH_P_IP)?;
        let sniff6 = sniff::sniff_socket(wire::ETH_P_IPV6).ok();
        Ok(Self {
            marker,
            mtu,
            send4,
            send6,
            sniff4,
            sniff6,
            reply_id: Mutex::new(HashMap::new()),
            _kernel_stays_quiet: super::nftguard::Guard::install(marker),
            wake: Wake::new()?,
        })
    }

    pub fn as_raw_fds(&self) -> Vec<std::os::fd::RawFd> {
        let mut fds = vec![self.sniff4.as_raw_fd()];
        if let Some(s) = &self.sniff6 {
            fds.push(s.as_raw_fd());
        }
        fds
    }

    fn id_the_client_will_accept(&self, dest: IpAddr) -> Option<u16> {
        self.reply_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&dest)
            .copied()
    }

    pub fn send_reply(&self, dest: IpAddr, payload: &[u8]) -> io::Result<()> {
        let Some(id) = self.id_the_client_will_accept(dest) else {
            return Ok(());
        };
        let body = wire::build_echo_reply(id, self.marker, payload, dest.is_ipv6());
        let addr: SockAddr = SocketAddr::new(dest, 0).into();
        let sock = match dest {
            IpAddr::V4(_) => &self.send4,
            IpAddr::V6(_) => self
                .send6
                .as_ref()
                .ok_or_else(|| io::Error::other("IPv6 send socket unavailable"))?,
        };
        sock.send_to(&body, &addr).map(|_| ())
    }

    pub fn recv_request(&self, buf: &mut [u8]) -> io::Result<Option<(IpAddr, Vec<u8>)>> {
        if let Ready::Woken = self.wake.wait_for_carrier_or_a_raise(&self.as_raw_fds())? {
            return Ok(None);
        }
        let sockets: [(&Socket, u16); 2] = [
            (&self.sniff4, wire::ETH_P_IP),
            (
                self.sniff6.as_ref().unwrap_or(&self.sniff4),
                wire::ETH_P_IPV6,
            ),
        ];
        for (sock, ethertype) in sockets {
            let Some((pkt, is_ingress)) = sniff::recvfrom_skipping_outgoing(sock, buf)? else {
                continue;
            };
            if !is_ingress {
                continue;
            }
            if let Some(p) = wire::extract_request(ethertype, pkt, self.marker) {
                self.reply_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(p.src, p.id);
                return Ok(Some((p.src, p.payload.to_vec())));
            }
        }
        Ok(None)
    }
}

impl CarrierServer for IcmpServer {
    type ReplyTo = IpAddr;
    fn capacity_for(&self, reply_to: &Self::ReplyTo) -> usize {
        self.mtu
            - if reply_to.is_ipv6() {
                IPV6_OVERHEAD
            } else {
                IPV4_OVERHEAD
            }
    }
    fn send_response(&self, reply_to: &Self::ReplyTo, wire: &[u8]) -> io::Result<()> {
        self.send_reply(*reply_to, wire)
    }
    fn recv_request(&self, buf: &mut [u8]) -> io::Result<Option<(Self::ReplyTo, Vec<u8>)>> {
        self.recv_request(buf)
    }
    fn stop_receiving(&self) {
        self.wake.raise();
    }
    fn told_to_stop(&self) -> bool {
        self.wake.raised()
    }
}
