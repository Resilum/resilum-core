//! End-to-end egress probe: open a dedicated link, run one SOCKS5 CONNECT to a
//! neutral reference target, and time the reply. leviculum exposes no link RTT,
//! so link_rtt is taken from the SOCKS greeting round-trip (mesh only) and e2e
//! from the full reply; the caller derives the egress leg as their difference.

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{LinkHandle, Node as LevNode};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{Instant, timeout};

use crate::egress::Candidate;
use crate::egress::connect::dial;
use crate::link::{LinkMsg, LinkRouter};

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const PROBE_TARGETS_ENV: &str = "RESILUM_EGRESS_PROBE_TARGETS";
const DEFAULT_PROBE_TARGETS: [(Ipv4Addr, u16); 3] = [
    (Ipv4Addr::new(1, 1, 1, 1), 443),
    (Ipv4Addr::new(8, 8, 8, 8), 443),
    (Ipv4Addr::new(9, 9, 9, 9), 443),
];

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
            Self::Socks => socks_probe(handle, from_link, host, port).await,
        }
    }
}

/// Probe targets by precedence: explicit `cli` (from ConnectConfig) over the
/// `RESILUM_EGRESS_PROBE_TARGETS` env var over built-in anycast defaults.
pub fn resolve_targets(cli: &[(String, u16)]) -> Vec<(Ipv4Addr, u16)> {
    if !cli.is_empty() {
        let parsed: Vec<_> = cli
            .iter()
            .filter_map(|(h, p)| Some((h.parse().ok()?, *p)))
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    match std::env::var(PROBE_TARGETS_ENV) {
        Ok(raw) => {
            let parsed: Vec<_> = raw.split(',').filter_map(parse_one).collect();
            if parsed.is_empty() {
                DEFAULT_PROBE_TARGETS.to_vec()
            } else {
                parsed
            }
        }
        Err(_) => DEFAULT_PROBE_TARGETS.to_vec(),
    }
}

fn parse_one(entry: &str) -> Option<(Ipv4Addr, u16)> {
    let (host, port) = entry.trim().rsplit_once(':')?;
    let port: u16 = port.parse().ok()?;
    (port != 0).then_some(())?;
    Some((host.parse().ok()?, port))
}

/// Probe `candidate` against the first reachable target, or `None` if the link
/// cannot be opened or every target fails. Each target uses a fresh link: on the
/// egress a link maps one-to-one onto a session.
pub async fn e2e_probe(
    engine: &Arc<LevNode>,
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

async fn socks_probe(
    handle: &LinkHandle,
    from_link: &mut UnboundedReceiver<LinkMsg>,
    host: Ipv4Addr,
    port: u16,
) -> Option<Probe> {
    let mut req = vec![5, 1, 0]; // greeting: VER, NMETHODS=1, no-auth
    req.extend_from_slice(&[5, 1, 0, 1]); // CONNECT, RSV, ATYP=IPv4
    req.extend_from_slice(&host.octets());
    req.extend_from_slice(&port.to_be_bytes());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_port_and_rejects_junk() {
        assert_eq!(
            parse_one("1.1.1.1:443"),
            Some((Ipv4Addr::new(1, 1, 1, 1), 443))
        );
        assert_eq!(
            parse_one(" 9.9.9.9:53 "),
            Some((Ipv4Addr::new(9, 9, 9, 9), 53))
        );
        assert_eq!(parse_one("1.1.1.1:0"), None);
        assert_eq!(parse_one("host.name:443"), None);
        assert_eq!(parse_one("1.1.1.1"), None);
    }

    #[test]
    fn socks_services_have_a_strategy_others_do_not() {
        assert!(ProbeStrategy::for_service("socks-egress").is_some());
        assert!(ProbeStrategy::for_service("tor").is_some());
        assert!(ProbeStrategy::for_service("i2p").is_some());
        assert!(ProbeStrategy::for_service("yggdrasil").is_none());
    }
}
