//! Shared byte relay: pump a stream against one established link until either
//! side ends. Callers own dialling and per-link cleanup around it.

use leviculum_std::api::LinkHandle;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::link::LinkMsg;
use crate::pump::pump;

pub(crate) async fn relay<S>(handle: &LinkHandle, from_link: UnboundedReceiver<LinkMsg>, stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (to_link, mut to_link_rx) = mpsc::unbounded_channel();
    let pumping = tokio::spawn(pump(stream, from_link, to_link));
    while let Some(bytes) = to_link_rx.recv().await {
        if handle.send(&bytes).await.is_err() {
            break;
        }
    }
    let _ = pumping.await;
}
