//! Discovered egress candidates and the registry that holds them.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub dest_hash: Vec<u8>,
    pub service: String,
    /// Exit country; `*` means unknown.
    pub exit_country: String,
    pub capabilities: Vec<String>,
    /// Mesh leg latency (consumer → provider), seconds.
    pub link_rtt: Option<f64>,
    /// Egress leg latency (provider → internet), seconds.
    pub egress_side: Option<f64>,
    pub healthy: bool,
    pub last_probe: Option<f64>,
}

impl Candidate {
    pub fn new(dest_hash: Vec<u8>, service: impl Into<String>) -> Self {
        Self {
            dest_hash,
            service: service.into(),
            exit_country: "*".into(),
            capabilities: Vec::new(),
            link_rtt: None,
            egress_side: None,
            healthy: true,
            last_probe: None,
        }
    }

    /// Total latency (mesh + egress), or `None` until both legs are probed.
    pub fn effective_latency(&self) -> Option<f64> {
        Some(self.link_rtt? + self.egress_side?)
    }
}

type ByService = HashMap<String, HashMap<Vec<u8>, Candidate>>;

#[derive(Default)]
pub struct CandidateRegistry {
    by_service: Mutex<ByService>,
}

impl CandidateRegistry {
    pub fn upsert(&self, service: &str, dest_hash: Vec<u8>, exit_country: &str, caps: Vec<String>) {
        let mut map = self.by_service.lock().expect("registry lock");
        let cand = map
            .entry(service.to_owned())
            .or_default()
            .entry(dest_hash.clone())
            .or_insert_with(|| Candidate::new(dest_hash, service));
        cand.exit_country = exit_country.to_owned();
        cand.capabilities = caps;
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
}
