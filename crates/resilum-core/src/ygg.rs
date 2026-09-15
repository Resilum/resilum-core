//! RNS-over-Yggdrasil transport. The caller runs the Yggdrasil engine
//! (`IfName=none`) and hands us its packet fd; a userspace TCP/IP stack over that
//! conduit both accepts RNS links arriving over ygg (→ a leviculum byte-channel)
//! and, through a local SOCKS proxy, dials ygg peers on the node's behalf — the
//! same links a node with an ygg tun gets, without needing the tun.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};

use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio_smoltcp::Net;

use crate::discovery::OriginRegistry;
use crate::letting_go::OnTheWayOut as _;

mod accept;
mod device;
mod socks;

type Links = Arc<Mutex<HashMap<IpAddr, ByteChannelHandle>>>;

/// A live Yggdrasil attachment; drop or [`YggHandle::detach`] to tear it down
/// and close the conduit fd.
#[must_use]
pub struct YggHandle {
    tasks: Vec<resilum_tasks::Watched>,
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
            self.engine
                .remove_interface(handle.id())
                .on_the_way_out("a yggdrasil link's interface");
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

#[non_exhaustive]
pub struct Attaching<'what> {
    pub engine: Arc<ReticulumNode>,
    pub origin: Arc<OriginRegistry>,
    pub ygg_fd: RawFd,
    pub ygg_address: &'what str,
    pub rns_port: u16,
    pub socks_port: Option<u16>,
    pub on_detach: Box<dyn FnOnce() + Send>,
    pub conns: Arc<resilum_tasks::Nursery>,
}

pub fn attach(attaching: Attaching<'_>) -> std::io::Result<YggHandle> {
    let Attaching {
        engine,
        origin,
        ygg_fd,
        ygg_address,
        rns_port,
        socks_port,
        on_detach,
        conns,
    } = attaching;
    let address: Ipv6Addr = ygg_address
        .parse()
        .map_err(|_| std::io::Error::other(format!("invalid ygg address: {ygg_address}")))?;
    let net = Arc::new(device::build_net(ygg_fd, address)?);

    let links: Links = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = vec![resilum_tasks::watch(
        "ygg: taking links over the overlay",
        accept::accept(
            engine.clone(),
            origin,
            net.clone(),
            address,
            rns_port,
            links.clone(),
        ),
    )];
    if let Some(port) = socks_port {
        tasks.push(resilum_tasks::watch(
            "ygg: the socks proxy onto the overlay",
            socks::serve(net.clone(), port, conns),
        ));
    }
    Ok(YggHandle {
        tasks,
        _net: net,
        links,
        engine,
        on_detach: Some(on_detach),
    })
}
