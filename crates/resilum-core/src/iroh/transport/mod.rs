//! A protected UDP custom transport for iroh (mobile). iroh's built-in UDP
//! socket is out of reach of our socket-protect hook, so under a captured tun it
//! would loop; here we bind the socket ourselves, run the hook on its fd (as
//! tor/ygg do), and hand iroh a datagram transport over it. Relay is kept, so
//! the node stays a full participant — only the direct path changes.

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use iroh::endpoint::transports::{
    CustomEndpoint, CustomSender, CustomTransport, RecvInfo, Transmit,
};
use iroh::{Endpoint, SecretKey};
use iroh_base::CustomAddr;
use leviculum_std::socket_hook::OutboundSocketHook;
use n0_watcher::Watchable;
use tokio::io::ReadBuf;
use tokio::net::UdpSocket;

use crate::config::IrohConfig;

mod addr;
use addr::{TRANSPORT_ID, from_custom, to_custom};

/// Build an endpoint whose direct path rides a protected UDP socket, keeping
/// relay and address lookup from [`super::engine::base_builder`].
pub async fn build_protected(
    secret: SecretKey,
    cfg: &IrohConfig,
    hook: OutboundSocketHook,
) -> Result<Endpoint, String> {
    super::engine::base_builder(secret, cfg)?
        .clear_ip_transports()
        .add_custom_transport(Arc::new(ProtectedUdp { hook }))
        .bind()
        .await
        .map_err(|e| e.to_string())
}

struct ProtectedUdp {
    hook: OutboundSocketHook,
}

impl std::fmt::Debug for ProtectedUdp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProtectedUdp")
    }
}

impl CustomTransport for ProtectedUdp {
    fn bind(&self) -> io::Result<Box<dyn CustomEndpoint>> {
        let socket = std::net::UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))
            .or_else(|_| std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)))?;
        (self.hook)(socket.as_raw_fd());
        socket.set_nonblocking(true)?;
        let local = socket.local_addr()?;
        Ok(Box::new(UdpEndpoint {
            local: Watchable::new(vec![to_custom(local)]),
            socket: Arc::new(UdpSocket::from_std(socket)?),
        }))
    }
}

#[derive(Debug)]
struct UdpEndpoint {
    socket: Arc<UdpSocket>,
    local: Watchable<Vec<CustomAddr>>,
}

impl CustomEndpoint for UdpEndpoint {
    fn watch_local_addrs(&self) -> n0_watcher::Direct<Vec<CustomAddr>> {
        self.local.watch()
    }

    fn create_sender(&self) -> Arc<dyn CustomSender> {
        Arc::new(UdpSender {
            socket: self.socket.clone(),
        })
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut<'_>],
        metas: &mut [noq_udp::RecvMeta],
        recv_infos: &mut [RecvInfo],
    ) -> Poll<io::Result<usize>> {
        if bufs.is_empty() {
            return Poll::Ready(Ok(0));
        }
        // One datagram per call; iroh re-polls for the rest of the batch.
        let mut read = ReadBuf::new(&mut bufs[0]);
        let from = ready!(self.socket.poll_recv_from(cx, &mut read))?;
        let len = read.filled().len();
        metas[0].len = len;
        metas[0].stride = len;
        recv_infos[0] = RecvInfo::new(to_custom(from), None);
        Poll::Ready(Ok(1))
    }
}

#[derive(Debug)]
struct UdpSender {
    socket: Arc<UdpSocket>,
}

impl CustomSender for UdpSender {
    fn is_valid_send_addr(&self, addr: &CustomAddr) -> bool {
        addr.id() == TRANSPORT_ID
    }

    fn poll_send(
        &self,
        cx: &mut Context,
        dst: &CustomAddr,
        _src: Option<&CustomAddr>,
        transmit: &Transmit<'_>,
    ) -> Poll<io::Result<()>> {
        let Some(addr) = from_custom(dst) else {
            return Poll::Ready(Err(io::Error::other("invalid custom addr")));
        };
        // max_transmit_segments defaults to 1, so `contents` is one datagram.
        ready!(self.socket.poll_send_to(cx, transmit.contents, addr))?;
        Poll::Ready(Ok(()))
    }
}
