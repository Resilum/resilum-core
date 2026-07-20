//! Shared deserialize/validation helpers for the config specs.

use std::net::IpAddr;
use std::str::FromStr;

use regex::Regex;
use serde::Deserialize;

use crate::{Error, Result};

/// A field YAML may give as a single string or a list of strings.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum StringOrSeq {
    One(String),
    Many(Vec<String>),
}

impl StringOrSeq {
    pub(super) fn into_vec(self) -> Vec<String> {
        match self {
            StringOrSeq::One(s) => vec![s],
            StringOrSeq::Many(v) => v,
        }
    }
}

/// Expand `${VAR}` / `${VAR:-default}` from the environment; missing with no
/// default → empty.
pub(super) fn expand_env(text: &str) -> String {
    let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)(?::-([^}]*))?\}").expect("valid regex");
    re.replace_all(text, |c: &regex::Captures| {
        std::env::var(&c[1]).unwrap_or_else(|_| c.get(2).map_or("", |m| m.as_str()).to_owned())
    })
    .into_owned()
}

/// `host:port` with a non-empty host and port in 1..=65535; host may be a
/// hostname, not necessarily an IP.
pub(super) fn require_host_port(ctx: &str, value: &str) -> Result<()> {
    if let Some((host, port)) = value.rsplit_once(':')
        && !host.is_empty()
        && port.parse::<u16>().is_ok_and(|p| p > 0)
    {
        return Ok(());
    }
    Err(Error::Config(format!("{ctx}: `{value}` must be host:port")))
}

/// Validate a CIDR like `10.20.0.0/24`.
pub(super) fn require_cidr(ctx: &str, value: &str) -> Result<()> {
    if let Some((ip, prefix)) = value.split_once('/')
        && let Ok(addr) = IpAddr::from_str(ip)
    {
        let max = if addr.is_ipv4() { 32 } else { 128 };
        if prefix.parse::<u8>().is_ok_and(|p| p <= max) {
            return Ok(());
        }
    }
    Err(Error::Config(format!(
        "{ctx}: `{value}` must be valid CIDR"
    )))
}

/// Validate a bare IP address.
pub(super) fn require_ip(ctx: &str, value: &str) -> Result<()> {
    IpAddr::from_str(value)
        .map(drop)
        .map_err(|_| Error::Config(format!("{ctx}: `{value}` is not an IP")))
}

/// Validate an MTU (68..=65535).
pub(super) fn require_mtu(ctx: &str, mtu: u32) -> Result<()> {
    if (68..=65535).contains(&mtu) {
        Ok(())
    } else {
        Err(Error::Config(format!("{ctx}: mtu {mtu} out of 68..=65535")))
    }
}

/// Validate a bitrate hint (1000..=1_000_000_000).
pub(super) fn require_bitrate(ctx: &str, bitrate: u64) -> Result<()> {
    if (1000..=1_000_000_000).contains(&bitrate) {
        Ok(())
    } else {
        Err(Error::Config(format!(
            "{ctx}: bitrate {bitrate} out of range"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_port_accepts_hostname_rejects_bad_port() {
        assert!(require_host_port("x", "example.com:9034").is_ok());
        assert!(require_host_port("x", "[::]:4242").is_ok());
        assert!(require_host_port("x", "host:0").is_err());
        assert!(require_host_port("x", "host:99999").is_err());
        assert!(require_host_port("x", "noport").is_err());
    }

    #[test]
    fn cidr_and_ip_checks() {
        assert!(require_cidr("x", "10.20.0.0/24").is_ok());
        assert!(require_cidr("x", "10.0.0.0/40").is_err());
        assert!(require_ip("x", "192.0.2.1").is_ok());
        assert!(require_ip("x", "nope").is_err());
    }
}
