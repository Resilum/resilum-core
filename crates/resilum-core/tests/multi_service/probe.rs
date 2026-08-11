//! Reaching one service and reading back which target answered.

use std::sync::Arc;
use std::time::Duration;

use leviculum_std::NodeEvent;
use leviculum_std::api::Destination;
use leviculum_std::driver::ReticulumNode;
use resilum_core::dispatch::Events;
use tokio::time::timeout;

/// Connect to `service`'s destination, send a byte, and return the first byte of
/// the reply (the target's tag).
pub async fn probe_service(engine: &Arc<ReticulumNode>, events: &Events, service: &str) -> u8 {
    let mut ev = events.subscribe();
    let want = Destination::compute_name_hash("resilum", &["bridge", "tcp", service]);

    let (dest_hash, key) = timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(event) = ev.recv().await
                && let NodeEvent::AnnounceReceived { announce, .. } = &*event
                && announce.name_hash() == &want
            {
                let key: [u8; 32] = announce.public_key()[32..64].try_into().unwrap();
                break (*announce.destination_hash(), key);
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("discover {service}"));

    let handle = engine.connect(&dest_hash, &key).await.expect("connect");
    let link_id = *handle.link_id();

    timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(event) = ev.recv().await
                && let NodeEvent::LinkEstablished { link_id: id, .. } = &*event
                && *id == link_id
            {
                break;
            }
        }
    })
    .await
    .expect("established");

    handle.send(b"ping").await.expect("send");

    timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(event) = ev.recv().await
                && let NodeEvent::MessageReceived {
                    link_id: id, data, ..
                } = &*event
                && *id == link_id
            {
                break data[0];
            }
        }
    })
    .await
    .expect("reply")
}
