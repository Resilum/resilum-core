//! Config for a covert-carrier discovery plugin.

use std::path::PathBuf;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CovertDiscoveryService {
    pub carrier: String,
    pub identity_path: PathBuf,
    pub mtu: usize,
    /// Explicit publish addresses (IPv4/IPv6). Empty → auto-detect at runtime.
    pub addresses: Vec<String>,
}

impl CovertDiscoveryService {
    pub fn icmp(identity_path: PathBuf) -> Self {
        Self {
            carrier: "icmp".into(),
            identity_path,
            mtu: 1400,
            addresses: Vec::new(),
        }
    }

    pub fn service_name(&self) -> String {
        format!("covert_{}", self.carrier)
    }
}
