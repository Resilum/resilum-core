use super::{Config, CovertDiscoveryService, Specs, default_announce_interval};

impl Config {
    pub fn minimal(instance_name: impl Into<String>) -> Self {
        Self {
            instance_name: instance_name.into(),
            storage_path: None,
            listen: None,
            reachable_on: None,
            discovery_name: None,
            network_identity: None,
            identity_private_base64: None,
            bootstrap: Vec::new(),
            bootstrap_only: Vec::new(),
            discover_interfaces: true,
            udp: None,
            ble: None,
            wifi_group: None,
            i2p: None,
            iroh: None,
            lxmf: None,
            autoconnect_max: 5,
            egress: Vec::new(),
            ingress: None,
            advertised_mirrors: Vec::new(),
            rngit_destination_file: None,
            discovery: Vec::new(),
            covert_discovery: Vec::new(),
            discovery_announce_interval: default_announce_interval(),
            advertised_services: Vec::new(),
            specs: Specs::default(),
        }
    }

    pub fn default_network(instance_name: impl Into<String>) -> Self {
        use crate::defaults;
        Self {
            listen: Some(defaults::DEFAULT_LISTEN.into()),
            discovery_name: Some(defaults::DISCOVERY_NAME.into()),
            network_identity: Some(defaults::NETWORK_IDENTITY_FILE.into()),
            bootstrap: defaults::YGG_ANCHORS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            bootstrap_only: defaults::PUBLIC_ANCHORS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            covert_discovery: vec![CovertDiscoveryService::icmp()],
            ..Self::minimal(instance_name)
        }
    }
}
