use std::net::Ipv4Addr;

const PROBE_TARGETS_ENV: &str = "RESILUM_EGRESS_PROBE_TARGETS";
const DEFAULT_PROBE_TARGETS: [(Ipv4Addr, u16); 3] = [
    (Ipv4Addr::new(1, 1, 1, 1), 443),
    (Ipv4Addr::new(8, 8, 8, 8), 443),
    (Ipv4Addr::new(9, 9, 9, 9), 443),
];

/// Probe targets by precedence: explicit `cli` (from IngressConfig) over the
/// `RESILUM_EGRESS_PROBE_TARGETS` env var over built-in anycast defaults.
pub fn resolve_targets(cli: &[(String, u16)]) -> Vec<(Ipv4Addr, u16)> {
    if !cli.is_empty() {
        let parsed: Vec<_> = cli
            .iter()
            .filter_map(|(h, p)| Some((h.parse().ok()?, *p)))
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    match std::env::var(PROBE_TARGETS_ENV) {
        Ok(raw) => {
            let parsed: Vec<_> = raw.split(',').filter_map(parse_one).collect();
            if parsed.is_empty() {
                DEFAULT_PROBE_TARGETS.to_vec()
            } else {
                parsed
            }
        }
        Err(_) => DEFAULT_PROBE_TARGETS.to_vec(),
    }
}

fn parse_one(entry: &str) -> Option<(Ipv4Addr, u16)> {
    let (host, port) = entry.trim().rsplit_once(':')?;
    let port: u16 = port.parse().ok()?;
    (port != 0).then_some(())?;
    Some((host.parse().ok()?, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_port_and_rejects_junk() {
        assert_eq!(
            parse_one("198.18.0.1:443"),
            Some((Ipv4Addr::new(198, 18, 0, 1), 443))
        );
        assert_eq!(
            parse_one(" 198.18.0.2:53 "),
            Some((Ipv4Addr::new(198, 18, 0, 2), 53))
        );
        assert_eq!(parse_one("198.18.0.1:0"), None);
        assert_eq!(parse_one("host.name:443"), None);
        assert_eq!(parse_one("198.18.0.1"), None);
    }
}
