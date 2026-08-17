//! Pick the egress candidate: lowest effective latency among eligibles, with
//! hysteresis so the choice does not flap between near-equal candidates.

use super::candidate::Candidate;

const MIN_REL: f64 = 0.10; // challenger must be >10% faster ...
const MIN_ABS: f64 = 0.010; // ... AND >10 ms faster to displace the incumbent.

fn lowest(a: &&Candidate, b: &&Candidate) -> std::cmp::Ordering {
    a.effective_latency()
        .unwrap()
        .total_cmp(&b.effective_latency().unwrap())
}

/// Choose the best candidate; `current` is the incumbent, if any.
pub fn choose_best<'a>(
    eligible: &'a [Candidate],
    current: Option<&Candidate>,
) -> Option<&'a Candidate> {
    if eligible.is_empty() {
        return None;
    }
    let measured: Vec<&Candidate> = eligible
        .iter()
        .filter(|c| c.effective_latency().is_some())
        .collect();
    let Some(&fastest) = measured.iter().min_by(|a, b| lowest(a, b)) else {
        return eligible.first(); // nothing probed yet — any eligible will do
    };
    let incumbent = current
        .and_then(|cur| eligible.iter().find(|c| c.dest_hash == cur.dest_hash))
        .filter(|c| c.effective_latency().is_some());
    let Some(cur) = incumbent else {
        return Some(fastest);
    };
    if cur.dest_hash == fastest.dest_hash {
        return Some(cur);
    }
    let cur_lat = cur.effective_latency().unwrap();
    let gain = cur_lat - fastest.effective_latency().unwrap();
    if gain > MIN_ABS && gain / cur_lat > MIN_REL {
        Some(fastest)
    } else {
        Some(cur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(hash: u8, latency: Option<f64>) -> Candidate {
        let mut c = Candidate::new(vec![hash], "tor");
        if let Some(l) = latency {
            c.link_rtt = Some(l);
            c.egress_rtt = Some(0.0);
        }
        c
    }

    #[test]
    fn empty_yields_none() {
        assert!(choose_best(&[], None).is_none());
    }

    #[test]
    fn unmeasured_returns_any() {
        let c = [cand(1, None)];
        assert_eq!(choose_best(&c, None).unwrap().dest_hash, vec![1]);
    }

    #[test]
    fn picks_fastest_without_incumbent() {
        let c = [cand(1, Some(0.3)), cand(2, Some(0.1))];
        assert_eq!(choose_best(&c, None).unwrap().dest_hash, vec![2]);
    }

    #[test]
    fn hysteresis_defends_a_near_equal_incumbent() {
        let c = [cand(1, Some(0.100)), cand(2, Some(0.095))];
        let incumbent = cand(1, Some(0.100));
        // 5 ms / 5% gain is below both thresholds → keep the incumbent.
        assert_eq!(
            choose_best(&c, Some(&incumbent)).unwrap().dest_hash,
            vec![1]
        );
    }

    #[test]
    fn clearly_faster_challenger_wins() {
        let c = [cand(1, Some(0.300)), cand(2, Some(0.100))];
        let incumbent = cand(1, Some(0.300));
        assert_eq!(
            choose_best(&c, Some(&incumbent)).unwrap().dest_hash,
            vec![2]
        );
    }
}
