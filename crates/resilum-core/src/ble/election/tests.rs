use super::{Candidate, Election, Facts, Standing, Verdict};

const US: [u8; 16] = [1; 16];
const THEM: [u8; 16] = [2; 16];
const WELL_PAST_THE_HOLD_DOWN: u64 = super::HOLD_DOWN_MS * 2;

fn candidate(who: [u8; 16], battery_percent: u8, has_an_uplink: bool) -> Candidate {
    Candidate {
        who,
        facts: Facts {
            has_an_uplink,
            battery_percent,
            ..Facts::default()
        },
        can_host_at_all: true,
    }
}

fn hosting_since(now_ms: u64) -> Election {
    let mut election = Election::watching(US);
    let field = [candidate(US, 50, false)];
    election.consider(&field, now_ms);
    election.consider(&field, now_ms + super::CLAIM_DELAY_MS + 1);
    election
}

#[test]
fn a_winner_waits_out_the_claim_delay_before_raising_anything() {
    let mut election = Election::watching(US);
    let field = [candidate(US, 50, false)];

    assert_eq!(election.consider(&field, 0), Verdict::CarryOn);
    assert_eq!(election.consider(&field, 1_000), Verdict::CarryOn);
    assert_eq!(election.consider(&field, 9_000), Verdict::RaiseTheGroup);
}

#[test]
fn a_better_candidate_appearing_during_the_delay_takes_the_claim_away() {
    let mut election = Election::watching(US);
    election.consider(&[candidate(US, 50, false)], 0);

    let better = [candidate(US, 50, false), candidate(THEM, 50, true)];

    assert_eq!(election.consider(&better, 9_000), Verdict::CarryOn);
    assert_eq!(election.standing(), Standing::Watching);
}

#[test]
fn a_host_does_not_yield_to_a_challenger_that_is_merely_a_little_better() {
    let mut election = hosting_since(0);
    let barely_better = [candidate(US, 50, false), candidate(THEM, 60, false)];

    assert_eq!(
        election.consider(&barely_better, WELL_PAST_THE_HOLD_DOWN),
        Verdict::CarryOn
    );
}

#[test]
fn a_host_yields_once_the_hold_down_has_passed_and_the_margin_is_met() {
    let mut election = hosting_since(0);
    let far_better = [candidate(US, 50, false), candidate(THEM, 50, true)];

    assert_eq!(
        election.consider(&far_better, WELL_PAST_THE_HOLD_DOWN),
        Verdict::StandDown
    );
    assert_eq!(election.standing(), Standing::Watching);
}

#[test]
fn a_host_keeps_the_group_up_through_the_hold_down_even_when_beaten() {
    let mut election = hosting_since(0);
    let far_better = [candidate(US, 50, false), candidate(THEM, 50, true)];

    assert_eq!(election.consider(&far_better, 1_000), Verdict::CarryOn);
    assert!(matches!(election.standing(), Standing::Hosting { .. }));
}

#[test]
fn an_empty_field_leaves_a_watcher_watching() {
    let mut election = Election::watching(US);

    assert_eq!(election.consider(&[], 0), Verdict::CarryOn);
    assert_eq!(election.standing(), Standing::Watching);
}
