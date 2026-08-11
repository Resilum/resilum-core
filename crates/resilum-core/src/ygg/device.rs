//! The userspace TCP/IP stack over the conduit fd.

use std::net::Ipv6Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use tokio_smoltcp::device::AsyncCapture;
use tokio_smoltcp::smoltcp::iface::Config;
use tokio_smoltcp::smoltcp::phy::{DeviceCapabilities, Medium};
use tokio_smoltcp::smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};
use tokio_smoltcp::{Net, NetConfig};

/// Yggdrasil routes jumbo frames; size the stack and read buffer for the max.
pub(super) const MTU: usize = 65535;

pub(super) fn build_net(ygg_fd: RawFd, address: Ipv6Addr) -> std::io::Result<Net> {
    let owned = unsafe { OwnedFd::from_raw_fd(ygg_fd) };
    set_nonblocking(owned.as_raw_fd())?;
    let mut caps = DeviceCapabilities::default();
    caps.medium = Medium::Ip;
    caps.max_transmission_unit = MTU;
    let device = AsyncCapture::new(owned, recv_packet, send_packet, caps)?;
    // `200::/7` is treated as on-link: smoltcp emits the IP packet straight out
    // the conduit and Yggdrasil does the overlay routing beyond it.
    let config = NetConfig::new(
        Config::new(HardwareAddress::Ip),
        IpCidr::new(IpAddress::Ipv6(address), 7),
        vec![],
    );
    Ok(Net::new(device, config))
}

fn recv_packet(fd: &mut OwnedFd) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; MTU];
    let n = unsafe { libc::read(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) };
    if n < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buf.truncate(n as usize);
    Ok(buf)
}

fn send_packet(fd: &mut OwnedFd, pkt: &[u8]) -> std::io::Result<()> {
    let n = unsafe { libc::write(fd.as_raw_fd(), pkt.as_ptr().cast(), pkt.len()) };
    if n < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn set_nonblocking(fd: RawFd) -> std::io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
