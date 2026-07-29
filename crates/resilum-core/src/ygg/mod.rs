//! RNS-over-Yggdrasil transport. The app runs the gomobile Yggdrasil engine
//! (`IfName=none`) and hands us its packet fd; a userspace TCP/IP stack over that
//! conduit both accepts RNS links arriving over ygg (→ a leviculum byte-channel)
//! and, through a local SOCKS proxy, dials ygg peers on the node's behalf — the
//! same links a node with an ygg tun gets, without needing the tun.

mod socks;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Arc, Mutex};

use leviculum_std::api::Node as LevNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::task::JoinHandle;
use tokio_smoltcp::device::AsyncCapture;
use tokio_smoltcp::smoltcp::iface::Config;
use tokio_smoltcp::smoltcp::phy::{DeviceCapabilities, Medium};
use tokio_smoltcp::smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};
use tokio_smoltcp::{Net, NetConfig};

use crate::discovery::OriginRegistry;

/// Yggdrasil routes jumbo frames; size the stack and read buffer for the max.
const MTU: usize = 65535;

type Links = Arc<Mutex<HashMap<IpAddr, ByteChannelHandle>>>;

/// A live Yggdrasil attachment; drop or [`YggHandle::detach`] to tear it down
/// and close the conduit fd.
#[must_use]
pub struct YggHandle {
    tasks: Vec<JoinHandle<()>>,
    _net: Arc<Net>,
    links: Links,
    // Runs on teardown; deactivates the ygg discovery service (see `node::ygg_attach`).
    on_detach: Option<Box<dyn FnOnce() + Send>>,
}

impl YggHandle {
    /// Tear the transport down now instead of on drop.
    pub fn detach(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
        for task in std::mem::take(&mut self.tasks) {
            task.abort();
        }
        // Dropping each handle detaches its interface, so the accepted links
        // leave the node the moment the transport is off.
        self.links.lock().expect("ygg links").clear();
        if let Some(deactivate) = self.on_detach.take() {
            deactivate();
        }
    }
}

impl Drop for YggHandle {
    fn drop(&mut self) {
        self.teardown();
    }
}

/// Attach the Yggdrasil conduit `ygg_fd` (the engine's `address` is `ygg_address`).
/// Accepts RNS links on `rns_port`; if `socks_port` is set, runs a loopback SOCKS
/// proxy there so discovery can dial ygg peers. Must run inside the node runtime.
pub fn attach(
    engine: Arc<LevNode>,
    origin: Arc<OriginRegistry>,
    ygg_fd: RawFd,
    ygg_address: &str,
    rns_port: u16,
    socks_port: Option<u16>,
    on_detach: Box<dyn FnOnce() + Send>,
) -> std::io::Result<YggHandle> {
    let address: Ipv6Addr = ygg_address
        .parse()
        .map_err(|_| std::io::Error::other(format!("invalid ygg address: {ygg_address}")))?;
    let net = Arc::new(build_net(ygg_fd, address)?);

    let links: Links = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = vec![tokio::spawn(accept(
        engine,
        origin,
        net.clone(),
        address,
        rns_port,
        links.clone(),
    ))];
    if let Some(port) = socks_port {
        tasks.push(tokio::spawn(socks::serve(net.clone(), port)));
    }
    Ok(YggHandle {
        tasks,
        _net: net,
        links,
        on_detach: Some(on_detach),
    })
}

fn build_net(ygg_fd: RawFd, address: Ipv6Addr) -> std::io::Result<Net> {
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

async fn accept(
    engine: Arc<LevNode>,
    origin: Arc<OriginRegistry>,
    net: Arc<Net>,
    address: Ipv6Addr,
    rns_port: u16,
    links: Links,
) {
    let mut listener = match net
        .tcp_bind(SocketAddr::new(address.into(), rns_port))
        .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, "ygg tcp_bind failed");
            return;
        }
    };
    while let Ok((stream, remote)) = listener.accept().await {
        let ip = remote.ip();
        let name = format!("ygg[{ip}]");
        match engine.spawn_byte_channel(&name, stream) {
            Ok(handle) => {
                origin.record(handle.id(), "yggdrasil");
                tracing::info!(%name, "attached RNS peer over yggdrasil");
                // One link per peer: inserting drops any prior handle for this
                // address, detaching a stale link the peer is re-dialing over.
                links.lock().expect("ygg links").insert(ip, handle);
            }
            Err(e) => tracing::warn!(%name, error = %e, "ygg byte-channel attach failed"),
        }
    }
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
