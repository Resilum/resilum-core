//! Filter egress candidates by the own/others policy and country rules.

use super::candidate::Candidate;
use super::own::OwnExits;

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

/// Candidates the policy permits, healthy or not: a candidate that failed its
/// last probe must stay probeable or it can never come back.
pub fn allowed(
    candidates: &[Candidate],
    use_own: &str,
    allow_countries: &[String],
    deny_countries: &[String],
    ours: &OwnExits,
) -> Vec<Candidate> {
    candidates
        .iter()
        .filter(|c| !ours.is_ours(c) || own_allowed(&c.service, use_own))
        .filter(|c| country_allowed(&c.exit_country, allow_countries, deny_countries))
        .cloned()
        .collect()
}

pub fn eligible(
    candidates: &[Candidate],
    use_own: &str,
    allow_countries: &[String],
    deny_countries: &[String],
    ours: &OwnExits,
) -> Vec<Candidate> {
    allowed(candidates, use_own, allow_countries, deny_countries, ours)
        .into_iter()
        .filter(|c| c.healthy)
        .collect()
}

#[cfg(test)]
mod tests;
