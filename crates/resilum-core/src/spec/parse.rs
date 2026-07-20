//! Load `bridges.yaml` into validated specs.

use serde::Deserialize;

use super::bridge::{BridgeMode, BridgeSpec, RawBridge};
use super::common::expand_env;
use super::covert::{CovertSpec, RawCovert};
use super::vpn::{RawVpn, VpnSpec};
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
struct Doc {
    #[serde(default)]
    bridges: Vec<RawBridge>,
    #[serde(default)]
    vpn: Vec<RawVpn>,
    #[serde(default)]
    covert: Vec<RawCovert>,
}

/// The three validated spec families parsed from one config document.
#[derive(Debug, Default, Clone)]
pub struct Specs {
    pub bridges: Vec<BridgeSpec>,
    pub vpn: Vec<VpnSpec>,
    pub covert: Vec<CovertSpec>,
}

/// Parse env-expanded YAML into validated specs.
pub fn load(yaml: &str) -> Result<Specs> {
    let doc: Doc = serde_yaml_ng::from_str(&expand_env(yaml))
        .map_err(|e| Error::Config(format!("yaml: {e}")))?;
    let mut specs = Specs::default();
    for (i, raw) in doc.bridges.into_iter().enumerate() {
        specs.bridges.push(raw.into_spec(&format!("bridges[{i}]"))?);
    }
    for (i, raw) in doc.vpn.into_iter().enumerate() {
        specs.vpn.push(raw.into_spec(&format!("vpn[{i}]"))?);
    }
    for (i, raw) in doc.covert.into_iter().enumerate() {
        specs.covert.push(raw.into_spec(&format!("covert[{i}]"))?);
    }
    Ok(specs)
}

/// Identity paths of listen-bridges sharing a service with `spec` — the
/// announces `spec` (when connect-mode) treats as its own during discovery.
pub fn siblings_for(spec: &BridgeSpec, all: &[BridgeSpec]) -> Vec<String> {
    all.iter()
        .filter(|s| {
            s.mode == BridgeMode::Listen && s.services.iter().any(|svc| spec.services.contains(svc))
        })
        .map(|s| s.identity.clone())
        .collect()
}
