//! L3 routing hub: terminate the tun's TCP flows in a userspace netstack and
//! forward each to its real destination through the egress mesh.

mod fakedns;
mod flow;
#[cfg(feature = "i2p")]
mod i2p;
mod tun;
mod udp;

use std::os::fd::RawFd;
use std::sync::Arc;

use futures::StreamExt as _;
use netstack_smoltcp::{StackBuilder, TcpListener};

use self::fakedns::FakeDns;
use self::flow::FlowCtx;
#[cfg(feature = "i2p")]
pub use self::i2p::I2pConduit;
use crate::config::IngressConfig;
use crate::egress::{ActiveLinks, CandidateRegistry};
use crate::link::LinkRouter;

/// Inputs for a routing hub, assembled by the node from its running state.
#[non_exhaustive]
pub struct VpnParams {
    pub engine: Arc<leviculum_std::driver::ReticulumNode>,
    pub router: Arc<LinkRouter>,
    pub registry: Arc<CandidateRegistry>,
    pub policy: IngressConfig,
    pub own: crate::egress::own::OwnExits,
    pub mtu: usize,
    /// Packet fd of a host-managed Yggdrasil conduit; `200::/7` is routed to it.
    pub ygg_fd: Option<RawFd>,
    #[cfg(feature = "arti")]
    pub tor: Option<crate::tor::ArtiClient>,
    #[cfg(feature = "i2p")]
    pub i2p: Option<Arc<I2pConduit>>,
}

/// A live attachment; drop or [`VpnHandle::detach`] to tear it down and close
/// the tun fd.
#[must_use]
pub struct VpnHandle {
    tasks: Vec<resilum_tasks::Watched>,
}

impl VpnHandle {
    pub fn detach(mut self) {
        for task in std::mem::take(&mut self.tasks) {
            task.abort();
        }
    }
}

impl Drop for VpnHandle {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Attach a routing hub to `tun_fd`. Must run inside the node's tokio runtime.
pub fn attach(params: VpnParams, tun_fd: RawFd) -> std::io::Result<VpnHandle> {
    let VpnParams {
        engine,
        router,
        registry,
        policy,
        own,
        mtu,
        ygg_fd,
        #[cfg(feature = "arti")]
        tor,
        #[cfg(feature = "i2p")]
        i2p,
    } = params;
    let tun = Arc::new(tun::TunFd::new(tun_fd)?);
    let ygg = ygg_fd.map(tun::TunFd::new).transpose()?.map(Arc::new);
    let (stack, runner, udp_socket, tcp_listener) = StackBuilder::default()
        .enable_tcp(true)
        .enable_udp(true)
        .enable_icmp(false)
        .mtu(mtu)
        .build()?;
    let tcp_listener = tcp_listener.expect("tcp is enabled");
    let udp_socket = udp_socket.expect("udp is enabled");
    let (sink, stream) = stack.split();

    let fakedns = Arc::new(FakeDns::default());
    let ctx = Arc::new(FlowCtx {
        engine,
        router,
        registry,
        active: Arc::new(ActiveLinks::default()),
        policy,
        own,
        fakedns: fakedns.clone(),
        #[cfg(feature = "arti")]
        tor,
        #[cfg(feature = "i2p")]
        i2p,
    });

    let mut tasks = Vec::new();
    if let Some(runner) = runner {
        tasks.push(resilum_tasks::watch("vpn: the network stack", async move {
            if let Err(e) = runner.await {
                tracing::warn!(error = %e, "the vpn network stack stopped");
            }
        }));
    }
    tasks.push(resilum_tasks::watch(
        "vpn: tun into the stack",
        tun::tun_to_stack(tun.clone(), sink, ygg.clone(), mtu),
    ));
    tasks.push(resilum_tasks::watch(
        "vpn: stack back into the tun",
        tun::stack_to_tun(tun.clone(), stream),
    ));
    if let Some(ygg) = ygg {
        tasks.push(resilum_tasks::watch(
            "vpn: the overlay into the tun",
            tun::conduit_to_tun(ygg, tun, mtu),
        ));
    }
    tasks.push(resilum_tasks::watch(
        "vpn: udp flows",
        udp::serve(udp_socket, fakedns),
    ));
    tasks.push(resilum_tasks::watch(
        "vpn: taking tcp flows",
        accept(ctx, tcp_listener),
    ));
    Ok(VpnHandle { tasks })
}

async fn accept(ctx: Arc<FlowCtx>, mut listener: TcpListener) {
    let flows = resilum_tasks::Nursery::default();
    while let Some((stream, _local, remote)) = listener.next().await {
        flows.keep(
            format!("vpn: a flow to {remote}"),
            flow::serve(ctx.clone(), stream, remote),
        );
    }
}
