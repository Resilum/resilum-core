use crate::config::IngressConfig;
use crate::egress::own::OwnExits;
use crate::egress::{Candidate, CandidateRegistry, choose_best, eligible};

/// The exit a new connection should take, or `None` with the reason logged:
/// both the SOCKS port and the tun device answer that question the same way.
pub fn best_available(
    registry: &CandidateRegistry,
    policy: &IngressConfig,
    ours: &OwnExits,
    incumbent: Option<&Candidate>,
) -> Option<Candidate> {
    let announced = registry.all();
    let passed_the_filter = eligible(
        &announced,
        &policy.use_own,
        &policy.allow_country,
        &policy.deny_country,
        ours,
    );
    let held = incumbent.and_then(|held| {
        passed_the_filter
            .iter()
            .find(|c| c.dest_hash == held.dest_hash)
    });
    match choose_best(&passed_the_filter, held) {
        Some(chosen) => Some(chosen.clone()),
        None => {
            tracing::warn!(
                announced = announced.len(),
                passed_the_filter = passed_the_filter.len(),
                "nothing to reach the internet through"
            );
            None
        }
    }
}
