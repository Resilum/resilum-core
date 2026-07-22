//! Parsing helpers for public-listen environment variables.

pub(super) const YGG_ENV: &str = "RESILUM_YGG_PUBLIC_LISTEN";
pub(super) const RNS_ENV: &str = "RESILUM_RNS_LISTEN";

pub(super) fn parse_ygg_listen(value: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for raw in value.split(',') {
        let entry = raw.trim();
        if entry.is_empty() {
            return Err(format!("empty entry in {YGG_ENV}"));
        }
        let Some(rest) = entry
            .strip_prefix("tcp://")
            .or_else(|| entry.strip_prefix("tls://"))
        else {
            return Err(format!("missing tcp://|tls:// scheme: {entry:?}"));
        };
        let (host, port) = split_host_port(rest)?;
        if host.is_empty() || !valid_port(port) {
            return Err(format!("bad address: {entry:?}"));
        }
        out.push(entry.to_string());
    }
    Ok(out)
}

pub(super) fn parse_rns_listen(value: &str) -> Result<Vec<(String, u16)>, String> {
    let mut out = Vec::new();
    for raw in value.split(',') {
        let entry = raw.trim();
        if entry.is_empty() {
            return Err(format!("empty entry in {RNS_ENV}"));
        }
        let (host, port) = split_host_port(entry)?;
        if host.is_empty() || !valid_port(port) {
            return Err(format!("bad address: {entry:?}"));
        }
        out.push((host.to_string(), port.parse().unwrap()));
    }
    Ok(out)
}

fn split_host_port(entry: &str) -> Result<(&str, &str), String> {
    let entry = entry.trim();
    if let Some(rest) = entry.strip_prefix('[') {
        let (host, port) = rest
            .split_once("]:")
            .ok_or_else(|| format!("bad bracketed address: {entry:?}"))?;
        Ok((host, port))
    } else {
        entry
            .rsplit_once(':')
            .ok_or_else(|| format!("missing port: {entry:?}"))
    }
}

fn valid_port(port: &str) -> bool {
    port.parse::<u16>().is_ok_and(|p| p >= 1)
}
