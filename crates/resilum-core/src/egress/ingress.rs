//! Connect side: accept local TCP and forward each connection through the
//! fastest eligible egress candidate, chosen per connection and kept sticky.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use leviculum_std::api::{DestinationHash, LinkHandle, LinkId};
use leviculum_std::driver::ReticulumNode;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::config::IngressConfig;
use crate::egress::own::{self, OwnExits};
use crate::egress::{ActiveLinks, Candidate, CandidateRegistry, best_available};
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
    own: OwnExits,
) {
    let Ok(listener) = TcpListener::bind(&cfg.listen_tcp).await else {
        return;
    };
    if let Ok(addr) = listener.local_addr() {
        socks_port.store(addr.port(), Ordering::Relaxed);
    }
    let mut current: Option<Candidate> = None;
    while let Ok((tcp, _)) = listener.accept().await {
        match best_available(&registry, &cfg, &own, current.as_ref()) {
            Some(chosen) => {
                match own.target_of(&chosen) {
                    Some(target) => tokio::spawn(own_session(target.to_owned(), tcp)),
                    None => tokio::spawn(session(
                        engine.clone(),
                        router.clone(),
                        active.clone(),
                        chosen.clone(),
                        tcp,
                    )),
                };
                current = Some(chosen);
            }
            None => {
                tokio::spawn(turn_away(tcp, REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH));
            }
        }
    }
}

async fn own_session(target: String, tcp: TcpStream) {
    if let Err(e) = own::session(&target, tcp).await {
        tracing::warn!(%target, error = %e, "this node's own exit did not carry the connection");
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
    let opened = crate::link::dial(engine, router, &DestinationHash::new(bytes)).await;
    if opened.is_none() {
        tracing::warn!(
            service = %candidate.service,
            dest = %data_encoding::HEXLOWER.encode(&candidate.dest_hash),
            "the egress this traffic was routed to never came up"
        );
    }
    opened
}
