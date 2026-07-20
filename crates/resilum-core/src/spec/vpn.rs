//! VPN spec.

use serde::Deserialize;

use super::common::{require_cidr, require_mtu};
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VpnMode {
    Server,
    Client,
}

#[derive(Debug, Clone)]
pub struct VpnSpec {
    pub mode: VpnMode,
    pub identity: String,
    pub tun: Option<String>,
    pub subnet: Option<String>,
    pub uplink: Option<String>,
    pub mtu: Option<u32>,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawVpn {
    mode: VpnMode,
    identity: String,
    #[serde(default)]
    tun: Option<String>,
    #[serde(default)]
    subnet: Option<String>,
    #[serde(default)]
    uplink: Option<String>,
    #[serde(default)]
    mtu: Option<u32>,
    #[serde(default)]
    target: Option<String>,
}

impl RawVpn {
    pub(super) fn into_spec(self, ctx: &str) -> Result<VpnSpec> {
        if let Some(subnet) = &self.subnet {
            require_cidr(ctx, subnet)?;
        }
        if let Some(mtu) = self.mtu {
            require_mtu(ctx, mtu)?;
        }
        Ok(VpnSpec {
            mode: self.mode,
            identity: self.identity,
            tun: self.tun,
            subnet: self.subnet,
            uplink: self.uplink,
            mtu: self.mtu,
            target: self.target,
        })
    }
}
