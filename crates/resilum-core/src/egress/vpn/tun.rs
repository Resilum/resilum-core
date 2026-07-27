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

pub(super) async fn tun_to_stack<S>(tun: Arc<TunFd>, mut sink: S, mtu: usize)
where
    S: futures::Sink<AnyIpPktFrame> + Unpin,
{
    let mut buf = vec![0u8; mtu];
    loop {
        match tun.recv(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) if sink.send(buf[..n].to_vec()).await.is_err() => break,
            Ok(_) => {}
        }
    }
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
