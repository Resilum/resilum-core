//! Latency monitor: probe the top-k eligible candidates and write their
//! link_rtt/egress_rtt back to the registry, so the selector sees measured
//! effective_latency.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::driver::ReticulumNode;
use tokio::time::{Instant, sleep};

use crate::config::IngressConfig;
use crate::egress::probe::{ProbeStrategy, e2e_probe, resolve_targets};
use crate::egress::{Candidate, CandidateRegistry, eligible};
use crate::link::LinkRouter;

const TICK: Duration = Duration::from_secs(1);
const PROBE_INTERVAL: f64 = 60.0;
const TOP_K: usize = 3;

/// Egress leg = full round-trip minus the mesh leg, floored at zero.
pub fn egress_rtt_from_probe(e2e: f64, link_rtt: f64) -> f64 {
    (e2e - link_rtt).max(0.0)
}

fn rank_key(c: &Candidate) -> (u8, f64) {
    match c.effective_latency() {
        Some(latency) => (0, latency),
        None => (1, 0.0),
    }
}

/// Up to `k` candidates, measured-first by effective_latency then unmeasured.
pub fn top_k(mut candidates: Vec<Candidate>, k: usize) -> Vec<Candidate> {
    candidates.sort_by(|a, b| rank_key(a).partial_cmp(&rank_key(b)).unwrap());
    candidates.truncate(k);
    candidates
}

pub fn due_for_probe(c: &Candidate, now: f64, interval: f64) -> bool {
    c.last_probe.is_none_or(|last| now - last >= interval)
}

fn report(candidate: &Candidate, result: Option<(f64, f64)>) {
    let service = &candidate.service;
    let dest = data_encoding::HEXLOWER.encode(&candidate.dest_hash);
    match result {
        Some((link_rtt, egress_rtt)) => tracing::debug!(
            %service,
            %dest,
            link_rtt,
            egress_rtt,
            "an egress answered a probe and stays in the running"
        ),
        None => tracing::warn!(
            %service,
            %dest,
            "an egress failed its probe and will be skipped until it answers again"
        ),
    }
}

pub async fn run(
    engine: Arc<ReticulumNode>,
    router: Arc<LinkRouter>,
    registry: Arc<CandidateRegistry>,
    cfg: IngressConfig,
    skip: HashMap<String, HashSet<Vec<u8>>>,
) {
    let targets = resolve_targets(&cfg.probe_targets);
    let base = Instant::now();
    loop {
        sleep(TICK).await;
        let elig = eligible(
            &registry.all(),
            &cfg.use_own,
            &cfg.allow_country,
            &cfg.deny_country,
            &skip,
        );
        let now = base.elapsed().as_secs_f64();
        for c in top_k(elig, TOP_K) {
            // Services without a probe strategy are left unmeasured, not probed.
            let Some(strategy) = ProbeStrategy::for_service(&c.service) else {
                continue;
            };
            if !due_for_probe(&c, now, PROBE_INTERVAL) {
                continue;
            }
            let result = e2e_probe(&engine, &router, &c, &strategy, &targets)
                .await
                .map(|p| (p.link_rtt, egress_rtt_from_probe(p.e2e, p.link_rtt)));
            report(&c, result);
            registry.record_probe(
                &c.service,
                &c.dest_hash,
                result,
                base.elapsed().as_secs_f64(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(hash: u8, latency: Option<(f64, f64)>, last_probe: Option<f64>) -> Candidate {
        let mut c = Candidate::new(vec![hash], "e2e");
        if let Some((link_rtt, egress_rtt)) = latency {
            c.link_rtt = Some(link_rtt);
            c.egress_rtt = Some(egress_rtt);
        }
        c.last_probe = last_probe;
        c
    }

    #[test]
    fn egress_rtt_floors_at_zero() {
        assert!((egress_rtt_from_probe(0.30, 0.10) - 0.20).abs() < 1e-9);
        assert_eq!(egress_rtt_from_probe(0.05, 0.10), 0.0);
    }

    #[test]
    fn top_k_orders_measured_first_then_truncates() {
        let cands = vec![
            cand(1, None, None),
            cand(2, Some((0.10, 0.10)), None), // effective 0.20
            cand(3, Some((0.01, 0.01)), None), // effective 0.02
        ];
        let ranked = top_k(cands, 2);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].dest_hash, vec![3]); // fastest measured
        assert_eq!(ranked[1].dest_hash, vec![2]);
    }

    #[test]
    fn due_when_never_probed_or_stale() {
        assert!(due_for_probe(&cand(1, None, None), 100.0, 60.0));
        assert!(due_for_probe(&cand(1, None, Some(30.0)), 100.0, 60.0));
        assert!(!due_for_probe(&cand(1, None, Some(70.0)), 100.0, 60.0));
    }
}
