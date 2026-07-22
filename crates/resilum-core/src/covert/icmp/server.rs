//! Covert ICMP server side. Linux only.
//!
//! Send: `SOCK_RAW + IPPROTO_ICMP{V4,V6}` — the kernel adds the IP header.
//! Sniff: `AF_PACKET + SOCK_DGRAM` per ethertype with a cBPF filter that lets
//! only ICMP frames wake us. Locally-originated frames are dropped by
//! `sll_pkttype == PACKET_OUTGOING` so we never re-ingest our own echoes.
//! Callers must first install [`super::nftguard::Guard`] so the kernel does
//! not answer echo-requests with our id itself.

#![cfg(target_os = "linux")]

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::wire;

const ETH_P_IP: u16 = wire::ETH_P_IP;
const ETH_P_IPV6: u16 = wire::ETH_P_IPV6;

pub struct IcmpServer {
    ident: u16,
    send4: Socket,
    send6: Option<Socket>,
    sniff4: Socket,
    sniff6: Option<Socket>,
}

impl IcmpServer {
    pub fn new(ident: u16) -> io::Result<Self> {
        let send4 = raw_send(Domain::IPV4, Protocol::ICMPV4)?;
        let send6 = raw_send(Domain::IPV6, Protocol::ICMPV6).ok();
        let sniff4 = sniff_socket(ETH_P_IP)?;
        let sniff6 = sniff_socket(ETH_P_IPV6).ok();
        Ok(Self {
            ident,
            send4,
            send6,
            sniff4,
            sniff6,
        })
    }

    /// Sniff sockets, ready for a `select`/`poll` loop.
    pub fn as_raw_fds(&self) -> Vec<std::os::fd::RawFd> {
        let mut fds = vec![self.sniff4.as_raw_fd()];
        if let Some(s) = &self.sniff6 {
            fds.push(s.as_raw_fd());
        }
        fds
    }

    /// Build an echo-reply with `payload` and send it to `dest`.
    pub fn send_reply(&self, dest: IpAddr, payload: &[u8]) -> io::Result<()> {
        let body = wire::build_echo_reply(self.ident, payload, dest.is_ipv6());
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

    /// Read the next inbound echo-request across both v4/v6 sniff sockets,
    /// filtering to our tunnel id. Returns `Ok(None)` on skip (foreign echo,
    /// our own outgoing, or a truncated frame); errors propagate.
    pub fn recv_request(&self, buf: &mut [u8]) -> io::Result<Option<(IpAddr, Vec<u8>)>> {
        let sockets: [(&Socket, u16); 2] = [
            (&self.sniff4, ETH_P_IP),
            (self.sniff6.as_ref().unwrap_or(&self.sniff4), ETH_P_IPV6),
        ];
        for (sock, ethertype) in sockets {
            let Some((pkt, is_ours)) = recvfrom_skipping_outgoing(sock, buf)? else {
                continue;
            };
            if !is_ours {
                continue;
            }
            if let Some(p) = wire::extract_request(ethertype, pkt, self.ident) {
                return Ok(Some((p.src, p.payload.to_vec())));
            }
        }
        Ok(None)
    }
}

fn raw_send(domain: Domain, proto: Protocol) -> io::Result<Socket> {
    Socket::new(domain, Type::RAW, Some(proto))
}

/// AF_PACKET + SOCK_DGRAM per ethertype with an inline cBPF filter for ICMP.
fn sniff_socket(ethertype: u16) -> io::Result<Socket> {
    let proto = Protocol::from(i32::from(ethertype.to_be()));
    let sock = Socket::new(Domain::PACKET, Type::DGRAM, Some(proto))?;
    attach_icmp_filter(&sock, ethertype)?;
    Ok(sock)
}

/// cBPF: match ICMP/ICMPv6 only. On AF_PACKET SOCK_DGRAM the L2 header is
/// stripped, so we start at the IP header (protocol byte 9 for IPv4,
/// next-header byte 6 for IPv6).
fn attach_icmp_filter(sock: &Socket, ethertype: u16) -> io::Result<()> {
    let (offset, value): (u32, u32) = match ethertype {
        ETH_P_IP => (9, 1),
        ETH_P_IPV6 => (6, 58),
        _ => return Err(io::Error::other("unsupported ethertype")),
    };
    let prog: [libc::sock_filter; 4] = [
        libc::sock_filter {
            code: 0x30,
            jt: 0,
            jf: 0,
            k: offset,
        }, // ldb [offset]
        libc::sock_filter {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: value,
        }, // jeq
        libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0x0000_FFFF,
        }, // ret accept
        libc::sock_filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0,
        }, // ret reject
    ];
    let fprog = libc::sock_fprog {
        len: prog.len() as u16,
        filter: prog.as_ptr() as *mut _,
    };
    let rc = unsafe {
        libc::setsockopt(
            sock.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_ATTACH_FILTER,
            (&fprog as *const libc::sock_fprog).cast(),
            std::mem::size_of::<libc::sock_fprog>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `recvfrom` with a `sockaddr_ll` so we can drop `PACKET_OUTGOING` (own
/// echoes we just sent). Returns `Ok(Some(payload, is_ingress))` on a frame.
fn recvfrom_skipping_outgoing<'a>(
    sock: &Socket,
    buf: &'a mut [u8],
) -> io::Result<Option<(&'a [u8], bool)>> {
    let mut sll: libc::sockaddr_ll = unsafe { std::mem::zeroed() };
    let mut sll_len = std::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
    let n = unsafe {
        libc::recvfrom(
            sock.as_raw_fd(),
            buf.as_mut_ptr().cast(),
            buf.len(),
            0,
            (&mut sll as *mut libc::sockaddr_ll).cast(),
            &mut sll_len,
        )
    };
    if n < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            return Ok(None);
        }
        return Err(err);
    }
    let is_ingress = sll.sll_pkttype != libc::PACKET_OUTGOING;
    Ok(Some((&buf[..n as usize], is_ingress)))
}
