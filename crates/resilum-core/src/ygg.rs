//! RNS-over-Yggdrasil transport. The caller runs the Yggdrasil engine
//! (`IfName=none`) and hands us its packet fd; a userspace TCP/IP stack over that
//! conduit both accepts RNS links arriving over ygg (→ a leviculum byte-channel)
//! and, through a local SOCKS proxy, dials ygg peers on the node's behalf — the
//! same links a node with an ygg tun gets, without needing the tun.

mod accept;
mod device;
mod socks;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};

use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::task::JoinHandle;
use tokio_smoltcp::Net;

use crate::discovery::OriginRegistry;

type Links = Arc<Mutex<HashMap<IpAddr, ByteChannelHandle>>>;

/// A live Yggdrasil attachment; drop or [`YggHandle::detach`] to tear it down
/// and close the conduit fd.
#[must_use]
pub struct YggHandle {
    tasks: Vec<JoinHandle<()>>,
    _net: Arc<Net>,
    links: Links,
    engine: Arc<ReticulumNode>,
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
        // The smoltcp stream over the now-closed conduit fd doesn't surface an
        // error, so the byte-channel task won't end on its own and dropping the
        // handle wouldn't detach it. Remove each interface explicitly so the
        // accepted links leave the node's status the moment the transport is off.
        let mut links = self.links.lock().expect("ygg links");
        for handle in links.values() {
            let _ = self.engine.remove_interface(handle.id());
        }
        links.clear();
        drop(links);
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
    engine: Arc<ReticulumNode>,
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
    let net = Arc::new(device::build_net(ygg_fd, address)?);

    let links: Links = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = vec![tokio::spawn(accept::accept(
        engine.clone(),
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
        engine,
        on_detach: Some(on_detach),
    })
}
