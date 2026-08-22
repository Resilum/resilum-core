//! Connect side: accept local TCP and forward each connection through the
//! fastest eligible egress candidate, chosen per connection and kept sticky.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use leviculum_std::api::{DestinationHash, LinkHandle, LinkId};
use leviculum_std::driver::ReticulumNode;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::config::IngressConfig;
use crate::egress::{ActiveLinks, Candidate, CandidateRegistry, choose_best, eligible};
use crate::link::{LinkMsg, LinkRouter};
use crate::socks5_tcp::{
    self, REP_EGRESS_DID_NOT_ANSWER, REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH,
};

pub async fn run(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    registry: Arc<CandidateRegistry>,
    active: Arc<ActiveLinks>,
    socks_port: Arc<AtomicU16>,
    cfg: IngressConfig,
    skip: HashMap<String, HashSet<Vec<u8>>>,
) {
    let Ok(listener) = TcpListener::bind(&cfg.listen_tcp).await else {
        return;
    };
    if let Ok(addr) = listener.local_addr() {
        socks_port.store(addr.port(), Ordering::Relaxed);
    }
    let mut current: Option<Vec<u8>> = None;
    while let Ok((tcp, _)) = listener.accept().await {
        let candidates = registry.all();
        let elig = eligible(
            &candidates,
            &cfg.use_own,
            &cfg.allow_country,
            &cfg.deny_country,
            &skip,
        );
        let incumbent = current
            .as_ref()
            .and_then(|h| elig.iter().find(|c| &c.dest_hash == h));
        match choose_best(&elig, incumbent) {
            Some(chosen) => {
                current = Some(chosen.dest_hash.clone());
                tokio::spawn(session(
                    engine.clone(),
                    router.clone(),
                    active.clone(),
                    chosen.clone(),
                    tcp,
                ));
            }
            None => {
                tracing::warn!(
                    announced = candidates.len(),
                    passed_the_filter = elig.len(),
                    "nothing to reach the internet through, turning a local connection away"
                );
                tokio::spawn(turn_away(tcp, REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH));
            }
        }
    }
}

async fn turn_away(mut tcp: TcpStream, why: u8) {
    if socks5_tcp::greet(&mut tcp).await.is_err() {
        return;
    }
    if socks5_tcp::read_connect(&mut tcp).await.is_err() {
        return;
    }
    let _ = socks5_tcp::reply(&mut tcp, why).await;
}

#[cfg(test)]
mod tests;

async fn session(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    active: Arc<ActiveLinks>,
    candidate: Candidate,
    tcp: TcpStream,
) {
    let Some((mut handle, link_id, from_link)) = dial(&engine, &router, &candidate).await else {
        tracing::warn!(
            service = %candidate.service,
            dest = %data_encoding::HEXLOWER.encode(&candidate.dest_hash),
            "the egress this connection was routed to never came up"
        );
        turn_away(tcp, REP_EGRESS_DID_NOT_ANSWER).await;
        return;
    };
    let dest_bytes: [u8; 16] = candidate
        .dest_hash
        .as_slice()
        .try_into()
        .expect("dial validated the hash length");
    active.register(dest_bytes, link_id);

    super::relay::relay(&handle, from_link, tcp).await;
    active.deregister(&dest_bytes, &link_id);
    router.detach(&link_id);
    let _ = handle.close().await;
}

pub(super) async fn dial(
    engine: &Arc<ReticulumNode>,
    router: &Arc<LinkRouter>,
    candidate: &Candidate,
) -> Option<(LinkHandle, LinkId, UnboundedReceiver<LinkMsg>)> {
    let bytes = <[u8; 16]>::try_from(candidate.dest_hash.as_slice()).ok()?;
    crate::link::dial(engine, router, &DestinationHash::new(bytes)).await
}
