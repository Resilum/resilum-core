use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

use super::LinkMsg;

pub async fn answered<T: DeserializeOwned>(
    from_link: &mut UnboundedReceiver<LinkMsg>,
    within: Duration,
) -> Option<T> {
    loop {
        match timeout(within, from_link.recv()).await {
            Ok(Some(LinkMsg::Data(bytes))) => return serde_json::from_slice(&bytes).ok(),
            Ok(Some(LinkMsg::Established)) => continue,
            _ => return None,
        }
    }
}
