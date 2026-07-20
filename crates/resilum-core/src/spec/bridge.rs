//! Connect/listen bridge spec.

use serde::Deserialize;

use super::common::{StringOrSeq, require_host_port};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeMode {
    Listen,
    Connect,
}

#[derive(Debug, Clone)]
pub struct BridgeSpec {
    pub mode: BridgeMode,
    pub services: Vec<String>,
    pub identity: String,
    pub tcp: String,
    pub target: Option<String>,
    pub use_own: String,
    pub allow_countries: Vec<String>,
    pub deny_countries: Vec<String>,
    pub probe_targets: Vec<String>,
    pub exit_country: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawBridge {
    mode: BridgeMode,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    services: Option<StringOrSeq>,
    identity: String,
    tcp: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    use_own: Option<String>,
    #[serde(default)]
    allow_countries: Vec<String>,
    #[serde(default)]
    deny_countries: Vec<String>,
    #[serde(default)]
    probe_targets: Vec<String>,
    #[serde(default)]
    exit_country: Option<String>,
}

impl RawBridge {
    pub(super) fn into_spec(self, ctx: &str) -> Result<BridgeSpec> {
        let services = self
            .services
            .map(StringOrSeq::into_vec)
            .or_else(|| self.service.map(|s| vec![s]))
            .unwrap_or_else(|| vec!["generic".into()]);
        if self.mode == BridgeMode::Listen && services.len() != 1 {
            return Err(Error::Config(format!(
                "{ctx}: listen-mode wraps exactly one service, got {}",
                services.len()
            )));
        }
        require_host_port(ctx, &self.tcp)?;
        Ok(BridgeSpec {
            mode: self.mode,
            services,
            identity: self.identity,
            tcp: self.tcp,
            target: self.target,
            use_own: self.use_own.unwrap_or_else(|| "smart".into()),
            allow_countries: self.allow_countries,
            deny_countries: self.deny_countries,
            probe_targets: self.probe_targets,
            exit_country: self.exit_country.unwrap_or_else(|| "*".into()),
        })
    }
}
