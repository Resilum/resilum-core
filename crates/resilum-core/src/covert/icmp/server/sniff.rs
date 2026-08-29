//! `AF_PACKET + SOCK_DGRAM` sniff with an inline cBPF filter for ICMP, plus
//! `recvfrom` on a `sockaddr_ll` so `PACKET_OUTGOING` (our own echoes) is
//! dropped. Linux only.

use std::io;
use std::os::fd::AsRawFd;

use socket2::{Domain, Protocol, Socket, Type};

use crate::covert::icmp::wire;

pub(super) fn sniff_socket(ethertype: u16) -> io::Result<Socket> {
    let proto = Protocol::from(i32::from(ethertype.to_be()));
    let sock = Socket::new(Domain::PACKET, Type::DGRAM, Some(proto))?;
    sock.set_nonblocking(true)?;
    attach_icmp_filter(&sock, ethertype)?;
    Ok(sock)
}

/// cBPF: match ICMP/ICMPv6 only. On `AF_PACKET SOCK_DGRAM` the L2 header is
/// stripped, so we start at the IP header (protocol byte 9 for IPv4,
/// next-header byte 6 for IPv6).
fn attach_icmp_filter(sock: &Socket, ethertype: u16) -> io::Result<()> {
    let (offset, value): (u32, u32) = match ethertype {
        wire::ETH_P_IP => (9, 1),
        wire::ETH_P_IPV6 => (6, 58),
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

/// Returns `Ok(Some((payload, is_ingress)))` on a frame; on `WouldBlock` returns
/// `Ok(None)`. `is_ingress = false` drops locally-originated frames.
pub(super) fn recvfrom_skipping_outgoing<'a>(
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
