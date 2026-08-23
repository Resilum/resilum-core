//! Discovered egress candidates and the registry that holds them.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub dest_hash: Vec<u8>,
    pub service: String,
    /// Exit country; `*` means unknown.
    pub exit_country: String,
    /// Consumer → provider, seconds.
    pub link_rtt: Option<f64>,
    /// Provider → internet, seconds.
    pub egress_rtt: Option<f64>,
    pub healthy: bool,
    pub last_probe: Option<f64>,
    failed_in_a_row: usize,
}

impl Candidate {
    pub fn new(dest_hash: Vec<u8>, service: impl Into<String>) -> Self {
        Self {
            dest_hash,
            service: service.into(),
            exit_country: "*".into(),
            link_rtt: None,
            egress_rtt: None,
            healthy: true,
            last_probe: None,
            failed_in_a_row: 0,
        }
    }

    /// Total latency (mesh + egress), or `None` until both legs are probed.
    pub fn effective_latency(&self) -> Option<f64> {
        Some(self.link_rtt? + self.egress_rtt?)
    }

    fn answered(&mut self, link_rtt: f64, egress_rtt: f64) {
        self.link_rtt = Some(settled(self.link_rtt, link_rtt));
        self.egress_rtt = Some(settled(self.egress_rtt, egress_rtt));
        self.healthy = true;
        self.failed_in_a_row = 0;
    }

    fn stayed_silent(&mut self) {
        self.failed_in_a_row += 1;
        self.healthy = self.failed_in_a_row < FAILURES_BEFORE_AN_EXIT_IS_WRITTEN_OFF;
    }
}

const FAILURES_BEFORE_AN_EXIT_IS_WRITTEN_OFF: usize = 3;
const WEIGHT_OF_THE_LATEST_MEASUREMENT: f64 = 0.3;

fn settled(so_far: Option<f64>, measured: f64) -> f64 {
    match so_far {
        Some(before) => {
            before * (1.0 - WEIGHT_OF_THE_LATEST_MEASUREMENT)
                + measured * WEIGHT_OF_THE_LATEST_MEASUREMENT
        }
        None => measured,
    }
}

type ByService = HashMap<String, HashMap<Vec<u8>, Candidate>>;

#[derive(Default)]
pub struct CandidateRegistry {
    by_service: Mutex<ByService>,
}

impl CandidateRegistry {
    /// Returns `true` when the candidate was newly discovered (not just refreshed).
    pub fn upsert(&self, service: &str, dest_hash: Vec<u8>, exit_country: &str) -> bool {
        let mut map = self.by_service.lock().expect("registry lock");
        let svc = map.entry(service.to_owned()).or_default();
        let is_new = !svc.contains_key(&dest_hash);
        let cand = svc
            .entry(dest_hash.clone())
            .or_insert_with(|| Candidate::new(dest_hash, service));
        cand.exit_country = exit_country.to_owned();
        is_new
    }

    pub fn remove(&self, service: &str, dest_hash: &[u8]) {
        if let Some(svc) = self
            .by_service
            .lock()
            .expect("registry lock")
            .get_mut(service)
        {
            svc.remove(dest_hash);
        }
    }

    pub fn for_service(&self, service: &str) -> Vec<Candidate> {
        let map = self.by_service.lock().expect("registry lock");
        map.get(service)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all(&self) -> Vec<Candidate> {
        let map = self.by_service.lock().expect("registry lock");
        map.values().flat_map(|m| m.values().cloned()).collect()
    }

    pub fn record_probe(
        &self,
        service: &str,
        dest_hash: &[u8],
        result: Option<(f64, f64)>,
        now: f64,
    ) {
        let mut map = self.by_service.lock().expect("registry lock");
        let Some(cand) = map.get_mut(service).and_then(|svc| svc.get_mut(dest_hash)) else {
            return;
        };
        match result {
            Some((link_rtt, egress_rtt)) => cand.answered(link_rtt, egress_rtt),
            None => cand.stayed_silent(),
        }
        cand.last_probe = Some(now);
    }
}

#[cfg(test)]
mod tests;
