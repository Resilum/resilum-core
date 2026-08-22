//! One intercepted TCP flow: route a `.onion` straight through Tor, everything
//! else through an eligible mesh egress via a SOCKS5 CONNECT.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use leviculum_std::driver::ReticulumNode;
use netstack_smoltcp::TcpStream;
use tokio::io::AsyncWriteExt;

use super::fakedns::FakeDns;
use crate::config::IngressConfig;
use crate::egress::socks5::Target;
use crate::egress::{
    ActiveLinks, CandidateRegistry, choose_best, eligible, ingress, relay, socks5,
};
use crate::link::LinkRouter;

pub(super) struct FlowCtx {
    pub engine: Arc<ReticulumNode>,
    pub router: Arc<LinkRouter>,
    pub registry: Arc<CandidateRegistry>,
    pub active: Arc<ActiveLinks>,
    pub policy: IngressConfig,
    pub own: crate::egress::own::OwnExits,
    pub fakedns: Arc<FakeDns>,
    #[cfg(feature = "arti")]
    pub tor: Option<crate::tor::ArtiClient>,
    #[cfg(feature = "i2p")]
    pub i2p: Option<Arc<super::i2p::I2pConduit>>,
}

/// A synthetic FakeDNS address carries the hostname really meant; hand
/// the egress the name so it resolves close to the exit. Real addresses pass
/// through literally.
fn target_for(fakedns: &FakeDns, dest: SocketAddr) -> Target {
    match dest.ip() {
        IpAddr::V4(v4) if FakeDns::is_fake(dest.ip()) => fakedns
            .resolve(v4)
            .map_or(Target::Addr(dest), |host| Target::Domain(host, dest.port())),
        _ => Target::Addr(dest),
    }
}

pub(super) async fn serve(ctx: Arc<FlowCtx>, stream: TcpStream, dest: SocketAddr) {
    let target = target_for(&ctx.fakedns, dest);
    #[cfg(feature = "arti")]
    if let Target::Domain(host, port) = &target
        && host.ends_with(".onion")
        && let Some(tor) = &ctx.tor
    {
        serve_onion(tor, host, *port, stream).await;
        return;
    }
    #[cfg(feature = "i2p")]
    if let Target::Domain(host, _) = &target
        && host.ends_with(".i2p")
        && let Some(i2p) = &ctx.i2p
    {
        serve_i2p(i2p, host, stream).await;
        return;
    }
    serve_mesh(ctx, stream, target, dest).await;
}

/// The TCP port is implicit for I2P eepsites, so it is not forwarded.
#[cfg(feature = "i2p")]
async fn serve_i2p(i2p: &super::i2p::I2pConduit, host: &str, mut stream: TcpStream) {
    match i2p.connect(host).await {
        Ok(mut upstream) => {
            let _ = tokio::io::copy_bidirectional(&mut stream, &mut upstream).await;
        }
        Err(e) => tracing::debug!(error = %e, host, "i2p dial failed"),
    }
}

#[cfg(feature = "arti")]
async fn serve_onion(tor: &crate::tor::ArtiClient, host: &str, port: u16, mut stream: TcpStream) {
    match tor.connect((host, port)).await {
        Ok(mut upstream) => {
            let _ = tokio::io::copy_bidirectional(&mut stream, &mut upstream).await;
        }
        Err(e) => tracing::debug!(error = %e, host, "onion dial failed"),
    }
}

async fn serve_mesh(ctx: Arc<FlowCtx>, mut stream: TcpStream, target: Target, dest: SocketAddr) {
    let candidates = ctx.registry.all();
    let elig = eligible(
        &candidates,
        &ctx.policy.use_own,
        &ctx.policy.allow_country,
        &ctx.policy.deny_country,
        &ctx.own,
    );
    let Some(chosen) = choose_best(&elig, None).cloned() else {
        return;
    };
    let Some((mut handle, link_id, mut from_link)) =
        ingress::dial(&ctx.engine, &ctx.router, &chosen).await
    else {
        return;
    };
    let dest_bytes: [u8; 16] = chosen
        .dest_hash
        .as_slice()
        .try_into()
        .expect("dial validated the hash length");
    ctx.active.register(dest_bytes, link_id);

    match socks5::connect(&handle, &mut from_link, &target).await {
        Ok(leftover) => {
            let flushed = leftover.is_empty() || stream.write_all(&leftover).await.is_ok();
            if flushed {
                relay::relay(&handle, from_link, stream).await;
            }
        }
        Err(e) => tracing::debug!(error = %e, dest = %dest, "vpn CONNECT failed"),
    }

    ctx.active.deregister(&dest_bytes, &link_id);
    ctx.router.detach(&link_id);
    let _ = handle.close().await;
}
