//! Connect side: accept local TCP and forward each connection through the
//! fastest eligible egress candidate, chosen per connection and kept sticky.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use leviculum_std::api::{DestinationHash, LinkHandle, LinkId, Node as LevNode};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::time::{Instant, timeout_at};

use crate::config::IngressConfig;
use crate::egress::{ActiveLinks, Candidate, CandidateRegistry, choose_best, eligible};
use crate::link::{LinkMsg, LinkRouter};
use crate::pump::pump;

const ESTABLISH_TIMEOUT: Duration = Duration::from_secs(30);
const PATH_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PATH_RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub async fn run(
    engine: Arc<LevNode>,
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
            None => drop(tcp),
        }
    }
}

async fn session(
    engine: Arc<LevNode>,
    router: Arc<LinkRouter>,
    active: Arc<ActiveLinks>,
    candidate: Candidate,
    tcp: TcpStream,
) {
    let Some((mut handle, link_id, from_link)) = dial(&engine, &router, &candidate).await else {
        return;
    };
    let dest_bytes: [u8; 16] = candidate
        .dest_hash
        .as_slice()
        .try_into()
        .expect("dial validated the hash length");
    active.register(dest_bytes, link_id);

    let (to_link, mut to_link_rx) = mpsc::unbounded_channel();
    let pumping = tokio::spawn(pump(tcp, from_link, to_link));
    while let Some(bytes) = to_link_rx.recv().await {
        if handle.send(&bytes).await.is_err() {
            break;
        }
    }
    let _ = pumping.await;
    active.deregister(&dest_bytes, &link_id);
    router.detach(&link_id);
    let _ = handle.close().await;
}

/// Open an established link to `candidate`, returning its handle, id and the
/// attached inbound receiver. The engine recalls the peer's identity from its
/// announce; the Ed25519 key is the second half of the public key.
pub(super) async fn dial(
    engine: &Arc<LevNode>,
    router: &Arc<LinkRouter>,
    candidate: &Candidate,
) -> Option<(LinkHandle, LinkId, UnboundedReceiver<LinkMsg>)> {
    let bytes = <[u8; 16]>::try_from(candidate.dest_hash.as_slice()).ok()?;
    let dest_hash = DestinationHash::new(bytes);
    if !engine.has_path(&dest_hash)
        && !engine
            .wait_for_path(&dest_hash, PATH_REQUEST_TIMEOUT, PATH_RETRY_INTERVAL)
            .await
            .unwrap_or(false)
    {
        return None;
    }
    let identity = engine.get_identity(&dest_hash)?;
    let signing_key = <[u8; 32]>::try_from(&identity.public_key_bytes()[32..64]).ok()?;
    let mut handle = engine
        .connect_with_key(&dest_hash, &signing_key)
        .await
        .ok()?;
    let link_id = *handle.link_id();
    let mut from_link = router.attach(link_id);
    if !wait_established(&mut from_link).await {
        router.detach(&link_id);
        let _ = handle.close().await;
        return None;
    }
    Some((handle, link_id, from_link))
}

pub(super) async fn wait_established(from_link: &mut mpsc::UnboundedReceiver<LinkMsg>) -> bool {
    let deadline = Instant::now() + ESTABLISH_TIMEOUT;
    loop {
        match timeout_at(deadline, from_link.recv()).await {
            Ok(Some(LinkMsg::Established)) => return true,
            Ok(Some(LinkMsg::Data(_))) => continue,
            _ => return false,
        }
    }
}
