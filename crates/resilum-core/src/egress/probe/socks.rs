//! The SOCKS5 exchange a probe times.

use std::net::Ipv4Addr;
use std::time::Duration;

use leviculum_std::api::LinkHandle;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{Instant, timeout};

use super::Probe;
use crate::link::LinkMsg;

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

pub(super) async fn socks_probe(
    handle: &LinkHandle,
    from_link: &mut UnboundedReceiver<LinkMsg>,
    host: Ipv4Addr,
    port: u16,
) -> Option<Probe> {
    let req = super::greeting_and_connect(host, port);

    let t0 = Instant::now();
    handle.send(&req).await.ok()?;

    let mut received = 0usize;
    let mut link_rtt: Option<f64> = None;
    loop {
        match timeout(PROBE_TIMEOUT, from_link.recv()).await {
            Ok(Some(LinkMsg::Data(chunk))) => {
                received += chunk.len();
                if link_rtt.is_none() && received >= 2 {
                    link_rtt = Some(t0.elapsed().as_secs_f64()); // greeting reply
                }
                if received >= 12 {
                    // + 10-byte IPv4 CONNECT reply
                    let e2e = t0.elapsed().as_secs_f64();
                    return Some(Probe {
                        link_rtt: link_rtt.unwrap_or(e2e),
                        e2e,
                    });
                }
            }
            _ => return None,
        }
    }
}
