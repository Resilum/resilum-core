//! Filter egress candidates by the own/others policy and country rules.

use std::collections::{HashMap, HashSet};

use super::candidate::Candidate;

/// Using your own exit for `socks-egress` would expose your real public IP with
/// no benefit, so `smart` excludes own candidates there (only there).
const SMART_OWN_EXCLUDED: &[&str] = &["socks-egress"];

fn own_allowed(service: &str, use_own: &str) -> bool {
    match use_own {
        "true" => true,
        "false" => false,
        _ => !SMART_OWN_EXCLUDED.contains(&service),
    }
}

fn country_allowed(country: &str, allow: &[String], deny: &[String]) -> bool {
    let filter_active = !allow.is_empty() || !deny.is_empty();
    if filter_active && country == "*" {
        return false; // an unknown exit cannot be guaranteed to avoid a forbidden country
    }
    if deny.iter().any(|c| c == country) {
        return false;
    }
    allow.is_empty() || allow.iter().any(|c| c == country)
}

/// `skip_hashes[service]` holds the dest hashes of our own listen-side bridges.
pub fn eligible(
    candidates: &[Candidate],
    use_own: &str,
    allow_countries: &[String],
    deny_countries: &[String],
    skip_hashes: &HashMap<String, HashSet<Vec<u8>>>,
) -> Vec<Candidate> {
    candidates
        .iter()
        .filter(|c| c.healthy)
        .filter(|c| {
            let is_own = skip_hashes
                .get(&c.service)
                .is_some_and(|s| s.contains(&c.dest_hash));
            !is_own || own_allowed(&c.service, use_own)
        })
        .filter(|c| country_allowed(&c.exit_country, allow_countries, deny_countries))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(hash: u8, service: &str, country: &str) -> Candidate {
        let mut c = Candidate::new(vec![hash], service);
        c.exit_country = country.into();
        c
    }

    fn own(service: &str, hash: u8) -> HashMap<String, HashSet<Vec<u8>>> {
        HashMap::from([(service.to_owned(), HashSet::from([vec![hash]]))])
    }

    #[test]
    fn unhealthy_is_dropped() {
        let mut c = cand(1, "tor", "*");
        c.healthy = false;
        assert!(eligible(&[c], "smart", &[], &[], &HashMap::new()).is_empty());
    }

    #[test]
    fn smart_excludes_own_socks_egress_only() {
        let socks = cand(1, "socks-egress", "*");
        let tor = cand(1, "tor", "*");
        assert!(eligible(&[socks], "smart", &[], &[], &own("socks-egress", 1)).is_empty());
        assert_eq!(eligible(&[tor], "smart", &[], &[], &own("tor", 1)).len(), 1);
    }

    #[test]
    fn use_own_true_and_false() {
        let c = cand(1, "socks-egress", "*");
        assert_eq!(
            eligible(
                std::slice::from_ref(&c),
                "true",
                &[],
                &[],
                &own("socks-egress", 1)
            )
            .len(),
            1
        );
        assert!(eligible(&[c], "false", &[], &[], &own("socks-egress", 1)).is_empty());
    }

    #[test]
    fn country_allow_deny_and_unknown() {
        let de = || vec!["DE".to_owned()];
        let unknown = cand(1, "tor", "*");
        let de_cand = cand(2, "tor", "DE");
        // deny DE
        assert!(
            eligible(
                std::slice::from_ref(&de_cand),
                "smart",
                &[],
                &de(),
                &HashMap::new()
            )
            .is_empty()
        );
        // allow only DE keeps DE, drops unknown
        assert_eq!(
            eligible(&[de_cand], "smart", &de(), &[], &HashMap::new()).len(),
            1
        );
        assert!(eligible(&[unknown], "smart", &de(), &[], &HashMap::new()).is_empty());
    }
}
