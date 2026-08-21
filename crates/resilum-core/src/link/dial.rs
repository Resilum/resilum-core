use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{DestinationHash, LinkHandle, LinkId};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{Instant, timeout_at};

use super::{LinkMsg, LinkRouter};

const ESTABLISH_TIMEOUT: Duration = Duration::from_secs(30);
const PATH_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PATH_RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub async fn dial(
    engine: &Arc<ReticulumNode>,
    router: &Arc<LinkRouter>,
    dest_hash: &DestinationHash,
) -> Option<(LinkHandle, LinkId, UnboundedReceiver<LinkMsg>)> {
    if !engine.has_path(dest_hash)
        && !engine
            .wait_for_path(dest_hash, PATH_REQUEST_TIMEOUT, PATH_RETRY_INTERVAL)
            .await
            .unwrap_or(false)
    {
        return None;
    }
    let identity = engine.get_identity(dest_hash)?;
    let signing_key = signing_half_of(&identity.public_key_bytes())?;
    let mut handle = engine.connect(dest_hash, &signing_key).await.ok()?;
    let link_id = *handle.link_id();
    let mut from_link = router.attach(link_id);
    if !established(&mut from_link).await {
        router.detach(&link_id);
        let _ = handle.close().await;
        return None;
    }
    Some((handle, link_id, from_link))
}

fn signing_half_of(public_key: &[u8]) -> Option<[u8; 32]> {
    <[u8; 32]>::try_from(public_key.get(32..64)?).ok()
}

pub async fn established(from_link: &mut UnboundedReceiver<LinkMsg>) -> bool {
    let deadline = Instant::now() + ESTABLISH_TIMEOUT;
    loop {
        match timeout_at(deadline, from_link.recv()).await {
            Ok(Some(LinkMsg::Established)) => return true,
            Ok(Some(LinkMsg::Data(_))) => continue,
            _ => return false,
        }
    }
}
