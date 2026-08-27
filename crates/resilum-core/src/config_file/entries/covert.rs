use std::path::PathBuf;

use serde::Deserialize;

use crate::config::CovertDiscoveryService;

#[derive(Deserialize)]
pub(in crate::config_file) struct CovertFile {
    #[serde(default = "icmp")]
    carrier: String,
    identity_path: PathBuf,
    #[serde(default)]
    mtu: Option<usize>,
    #[serde(default)]
    addresses: Vec<String>,
    #[serde(default)]
    dial_local_networks: bool,
}

fn icmp() -> String {
    "icmp".into()
}

impl From<CovertFile> for CovertDiscoveryService {
    fn from(f: CovertFile) -> Self {
        let mut svc = CovertDiscoveryService::icmp(f.identity_path);
        svc.carrier = f.carrier;
        if let Some(mtu) = f.mtu {
            svc.mtu = mtu;
        }
        svc.addresses = f.addresses;
        svc.dial_local_networks = f.dial_local_networks;
        svc
    }
}
