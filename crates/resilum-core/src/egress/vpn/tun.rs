//! Async packet I/O over a caller-supplied tun file descriptor, and the two
//! pumps that shuttle IP frames between it and the userspace netstack.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use netstack_smoltcp::AnyIpPktFrame;
use tokio::io::unix::AsyncFd;

/// Owns the tun fd for the lifetime of the attachment; dropping it closes the fd.
pub(super) struct TunFd {
    inner: AsyncFd<OwnedFd>,
}

impl TunFd {
    pub(super) fn new(fd: RawFd) -> io::Result<Self> {
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        set_nonblocking(owned.as_raw_fd())?;
        Ok(Self {
            inner: AsyncFd::new(owned)?,
        })
    }

    async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|fd| read_fd(fd.get_ref().as_raw_fd(), buf)) {
                Ok(res) => return res,
                Err(_would_block) => continue,
            }
        }
    }

    async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|fd| write_fd(fd.get_ref().as_raw_fd(), buf)) {
                Ok(res) => return res,
                Err(_would_block) => continue,
            }
        }
    }
}

/// Reads the tun, diverting `200::/7` to the `ygg` conduit and the rest to the
/// netstack.
pub(super) async fn tun_to_stack<S>(
    tun: Arc<TunFd>,
    mut sink: S,
    ygg: Option<Arc<TunFd>>,
    mtu: usize,
) where
    S: futures::Sink<AnyIpPktFrame> + Unpin,
{
    let mut buf = vec![0u8; mtu];
    loop {
        let pkt = match tun.recv(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => &buf[..n],
        };
        if let Some(ygg) = &ygg
            && is_yggdrasil(pkt)
        {
            if ygg.send(pkt).await.is_err() {
                break;
            }
        } else if sink.send(pkt.to_vec()).await.is_err() {
            break;
        }
    }
}

pub(super) async fn conduit_to_tun(conduit: Arc<TunFd>, tun: Arc<TunFd>, mtu: usize) {
    let mut buf = vec![0u8; mtu];
    loop {
        match conduit.recv(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) if tun.send(&buf[..n]).await.is_err() => break,
            Ok(_) => {}
        }
    }
}

/// IPv6 whose destination is in `200::/7`, the range Yggdrasil routes.
fn is_yggdrasil(pkt: &[u8]) -> bool {
    pkt.len() >= 40 && pkt[0] >> 4 == 6 && pkt[24] & 0xFE == 0x02
}

pub(super) async fn stack_to_tun<St>(tun: Arc<TunFd>, mut stream: St)
where
    St: futures::Stream<Item = io::Result<AnyIpPktFrame>> + Unpin,
{
    while let Some(Ok(pkt)) = stream.next().await {
        if tun.send(&pkt).await.is_err() {
            break;
        }
    }
}

fn read_fd(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
    usize::try_from(n).map_err(|_| io::Error::last_os_error())
}

fn write_fd(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
    let n = unsafe { libc::write(fd, buf.as_ptr().cast(), buf.len()) };
    usize::try_from(n).map_err(|_| io::Error::last_os_error())
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_yggdrasil;

    fn ipv6_to(dst_first: u8) -> Vec<u8> {
        let mut pkt = vec![0u8; 40];
        pkt[0] = 0x60;
        pkt[24] = dst_first;
        pkt
    }

    #[test]
    fn classifies_the_yggdrasil_range() {
        assert!(is_yggdrasil(&ipv6_to(0x02)));
        assert!(is_yggdrasil(&ipv6_to(0x03)));
        assert!(!is_yggdrasil(&ipv6_to(0x20))); // 2000::/3 public v6
    }

    #[test]
    fn rejects_ipv4_and_runts() {
        let mut v4 = ipv6_to(0x02);
        v4[0] = 0x45;
        assert!(!is_yggdrasil(&v4));
        assert!(!is_yggdrasil(&[0x60; 20]));
    }
}
