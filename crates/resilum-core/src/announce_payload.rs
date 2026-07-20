//! Announce payload: a small JSON envelope carrying the running version and,
//! optionally, endpoint / exit country / capabilities. Wire-compatible with the
//! bridge (`{"v":"0.0.0","ep":..,"co":..,"cap":[..]}`).

use serde::{Deserialize, Serialize};

const VERSION: &str = match option_env!("RESILUM_VERSION") {
    Some(v) => v,
    None => "0.0.0",
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAnnounce {
    pub version: String,
    pub endpoint: Option<Vec<u8>>,
    pub exit_country: String,
    pub capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Wire {
    v: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ep: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    co: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cap: Option<Vec<String>>,
}

pub fn pack(endpoint: Option<&[u8]>, exit_country: &str, capabilities: &[String]) -> Vec<u8> {
    let wire = Wire {
        v: VERSION.to_owned(),
        ep: endpoint.map(|e| String::from_utf8_lossy(e).into_owned()),
        co: (exit_country != "*" && !exit_country.is_empty()).then(|| exit_country.to_owned()),
        cap: (!capabilities.is_empty()).then(|| capabilities.to_vec()),
    };
    serde_json::to_vec(&wire).expect("announce payload serializes")
}

/// Decode an inbound announce. `None` means "drop this peer": missing,
/// malformed, or version-incompatible payload.
pub fn parse(raw: &[u8]) -> Option<ParsedAnnounce> {
    if raw.is_empty() {
        return None;
    }
    let wire: Wire = serde_json::from_slice(raw).ok()?;
    if !is_compatible(&wire.v) {
        return None;
    }
    Some(ParsedAnnounce {
        endpoint: wire.ep.map(String::into_bytes),
        exit_country: wire
            .co
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| "*".into()),
        capabilities: wire.cap.unwrap_or_default(),
        version: wire.v,
    })
}

/// Peers are compatible iff their semver majors match; any non-major bump may
/// change the wire format.
fn is_compatible(other: &str) -> bool {
    match (semver_major(VERSION), semver_major(other)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn semver_major(raw: &str) -> Option<u64> {
    let core = raw.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?;
    let (minor, patch) = (parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(minor) && digits(patch)).then_some(())?;
    major.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_minimal_and_full() {
        assert_eq!(pack(None, "*", &[]), br#"{"v":"0.0.0"}"#.to_vec());
        let full = pack(None, "US", &["ygg".into()]);
        assert_eq!(full, br#"{"v":"0.0.0","co":"US","cap":["ygg"]}"#.to_vec());
    }

    #[test]
    fn parses_country_and_caps() {
        let p = parse(br#"{"v":"0.0.0","co":"NL","cap":["x","y"]}"#).unwrap();
        assert_eq!(p.exit_country, "NL");
        assert_eq!(p.capabilities, vec!["x", "y"]);
        assert_eq!(p.endpoint, None);
    }

    #[test]
    fn missing_country_defaults_to_wildcard() {
        assert_eq!(parse(br#"{"v":"0.0.0"}"#).unwrap().exit_country, "*");
    }

    #[test]
    fn drops_incompatible_malformed_and_empty() {
        assert!(parse(br#"{"v":"1.0.0"}"#).is_none()); // different major
        assert!(parse(b"not json").is_none());
        assert!(parse(br#"{"co":"US"}"#).is_none()); // missing version
        assert!(parse(b"").is_none());
    }
}
