//! End-to-end egress probe: open a dedicated link, run one SOCKS5 CONNECT to a
//! neutral reference target, and time the reply. leviculum exposes no link RTT,
//! so link_rtt is taken from the SOCKS greeting round-trip (mesh only) and e2e
//! from the full reply; the caller derives the egress leg as their difference.

mod local;
mod socks;
mod targets;

use std::net::Ipv4Addr;
use std::sync::Arc;

use leviculum_std::api::LinkHandle;
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::UnboundedReceiver;

pub use local::over_a_local_socket;
pub use targets::resolve_targets;

fn greeting_and_connect(host: Ipv4Addr, port: u16) -> Vec<u8> {
    let mut req = vec![5, 1, 0, 5, 1, 0, 1];
    req.extend_from_slice(&host.octets());
    req.extend_from_slice(&port.to_be_bytes());
    req
}

use crate::egress::Candidate;
use crate::egress::ingress::dial;
use crate::link::{LinkMsg, LinkRouter};

/// One probe's raw timings, seconds: mesh round-trip and full round-trip.
pub struct Probe {
    pub link_rtt: f64,
    pub e2e: f64,
}

/// How a service's egress latency is measured. Extend with a variant (and a
/// `for_service` mapping) to latency-probe a non-SOCKS service.
pub enum ProbeStrategy {
    /// SOCKS5 CONNECT to a clearnet target (socks-egress, tor, i2p).
    Socks,
}

impl ProbeStrategy {
    /// The strategy for `service`, or `None` when it is not latency-probed.
    pub fn for_service(service: &str) -> Option<Self> {
        match service {
            "socks-egress" | "tor" | "i2p" => Some(Self::Socks),
            _ => None,
        }
    }

    async fn measure(
        &self,
        handle: &LinkHandle,
        from_link: &mut UnboundedReceiver<LinkMsg>,
        host: Ipv4Addr,
        port: u16,
    ) -> Option<Probe> {
        match self {
            Self::Socks => socks::socks_probe(handle, from_link, host, port).await,
        }
    }
}

/// Probe `candidate` against the first reachable target, or `None` if the link
/// cannot be opened or every target fails. Each target uses a fresh link: on the
/// egress a link maps one-to-one onto a session.
pub async fn e2e_probe(
    engine: &Arc<ReticulumNode>,
    router: &Arc<LinkRouter>,
    candidate: &Candidate,
    strategy: &ProbeStrategy,
    targets: &[(Ipv4Addr, u16)],
) -> Option<Probe> {
    for (host, port) in targets {
        let (mut handle, link_id, mut from_link) = dial(engine, router, candidate).await?;
        let result = strategy
            .measure(&handle, &mut from_link, *host, *port)
            .await;
        router.detach(&link_id);
        let _ = handle.close().await;
        if result.is_some() {
            return result;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socks_services_have_a_strategy_others_do_not() {
        assert!(ProbeStrategy::for_service("socks-egress").is_some());
        assert!(ProbeStrategy::for_service("tor").is_some());
        assert!(ProbeStrategy::for_service("i2p").is_some());
        assert!(ProbeStrategy::for_service("yggdrasil").is_none());
    }
}
