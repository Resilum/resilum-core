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
    /// Command template for the peer's client subprocess. Placeholders
    /// `{dst}`, `{server_identity}`, `{mtu}` are substituted per peer.
    pub client_command: String,
    /// Respawn delay for the spawned client PipeInterface.
    pub respawn_delay_secs: u64,
}

impl CovertDiscoveryService {
    pub fn icmp(identity_path: PathBuf) -> Self {
        Self {
            carrier: "icmp".into(),
            identity_path,
            mtu: 1400,
            addresses: Vec::new(),
            client_command:
                "resilumd covert icmp client --dst {dst} --server-identity {server_identity} --mtu {mtu}"
                    .into(),
            respawn_delay_secs: 5,
        }
    }

    pub fn service_name(&self) -> String {
        format!("covert_{}", self.carrier)
    }
}
