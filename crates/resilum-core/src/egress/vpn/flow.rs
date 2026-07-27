//! One intercepted TCP flow: pick an eligible egress, open a link, ask its
//! embedded backend to CONNECT to the flow's real destination, then relay.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

use leviculum_std::api::Node as LevNode;
use netstack_smoltcp::TcpStream;
use tokio::io::AsyncWriteExt;

use crate::config::IngressConfig;
use crate::egress::{
    ActiveLinks, CandidateRegistry, choose_best, eligible, ingress, relay, socks5,
};
use crate::link::LinkRouter;

pub(super) struct FlowCtx {
    pub engine: Arc<LevNode>,
    pub router: Arc<LinkRouter>,
    pub registry: Arc<CandidateRegistry>,
    pub active: Arc<ActiveLinks>,
    pub policy: IngressConfig,
    pub skip: HashMap<String, HashSet<Vec<u8>>>,
}

pub(super) async fn serve(ctx: Arc<FlowCtx>, mut stream: TcpStream, dest: SocketAddr) {
    let candidates = ctx.registry.all();
    let elig = eligible(
        &candidates,
        &ctx.policy.use_own,
        &ctx.policy.allow_country,
        &ctx.policy.deny_country,
        &ctx.skip,
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

    match socks5::connect(&handle, &mut from_link, dest).await {
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
