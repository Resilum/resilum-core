//! Covert-transport spec.

use serde::Deserialize;

use super::common::{StringOrSeq, require_bitrate, require_ip, require_mtu};
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct CovertSpec {
    pub carrier: String,
    pub role: String,
    pub addresses: Vec<String>,
    pub interface: String,
    pub mtu: u32,
    pub bitrate: u64,
    pub identity: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawCovert {
    carrier: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    address: Option<StringOrSeq>,
    #[serde(default)]
    addresses: Option<StringOrSeq>,
    #[serde(default)]
    interface: Option<String>,
    #[serde(default)]
    mtu: Option<u32>,
    #[serde(default)]
    bitrate: Option<u64>,
    #[serde(default)]
    identity: Option<String>,
}

impl RawCovert {
    pub(super) fn into_spec(self, ctx: &str) -> Result<CovertSpec> {
        if self.carrier.is_empty() {
            return Err(Error::Config(format!("{ctx}: missing carrier")));
        }
        let addresses = normalize_addresses(self.addresses.or(self.address));
        for addr in &addresses {
            require_ip(ctx, addr)?;
        }
        let mtu = self.mtu.unwrap_or(1400);
        let bitrate = self.bitrate.unwrap_or(32000);
        require_mtu(ctx, mtu)?;
        require_bitrate(ctx, bitrate)?;
        Ok(CovertSpec {
            carrier: self.carrier,
            role: self.role.unwrap_or_else(|| "both".into()),
            addresses,
            interface: self.interface.unwrap_or_default(),
            mtu,
            bitrate,
            identity: self.identity.unwrap_or_default(),
        })
    }
}

/// A list stays a list; a scalar is comma-split (the shape `${VAR}` expands to).
fn normalize_addresses(raw: Option<StringOrSeq>) -> Vec<String> {
    let items = match raw {
        None => return Vec::new(),
        Some(StringOrSeq::One(s)) => s.split(',').map(str::to_owned).collect(),
        Some(StringOrSeq::Many(v)) => v,
    };
    items
        .into_iter()
        .map(|a| a.trim().to_owned())
        .filter(|a| !a.is_empty())
        .collect()
}
