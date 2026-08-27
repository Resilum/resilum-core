//! Config for a covert-carrier discovery plugin.

use crate::discovery::covert::Reach;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CovertDiscoveryService {
    pub carrier: String,
    pub mtu: usize,
    /// Explicit publish addresses (IPv4/IPv6). Empty → auto-detect at runtime.
    pub addresses: Vec<String>,
    pub dial_local_networks: bool,
}

impl CovertDiscoveryService {
    pub fn icmp() -> Self {
        Self {
            carrier: "icmp".into(),
            mtu: 1400,
            addresses: Vec::new(),
            dial_local_networks: false,
        }
    }

    pub fn service_name(&self) -> String {
        format!("covert_{}", self.carrier)
    }

    pub fn reach(&self) -> Reach {
        if self.dial_local_networks {
            Reach::LocalNetworksToo
        } else {
            Reach::GlobalOnly
        }
    }
}
