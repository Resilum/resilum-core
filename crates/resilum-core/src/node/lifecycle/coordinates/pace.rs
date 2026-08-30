use std::time::Duration;

const WHILE_SETTLING: Duration = Duration::from_secs(20);
const ONCE_SETTLED: Duration = Duration::from_secs(900);
const SETTLED_BELOW: f64 = 0.25;
const ROUNDS_MISSED_BEFORE_FORGOTTEN: u32 = 3;
const NEVER_FORGET_SOONER_THAN: Duration = Duration::from_secs(600);
const A_LINK_IS_SLOW_ABOVE: Duration = Duration::from_secs(1);
const SLOW_LINKS_REASKED_NO_SOONER_THAN: Duration = Duration::from_secs(300);

pub(super) fn between_asks(our_error: f64) -> Duration {
    if our_error > SETTLED_BELOW {
        WHILE_SETTLING
    } else {
        ONCE_SETTLED
    }
}

fn reask_no_sooner_than(estimated_rtt: Option<Duration>) -> Duration {
    match estimated_rtt {
        Some(rtt) if rtt >= A_LINK_IS_SLOW_ABOVE => SLOW_LINKS_REASKED_NO_SOONER_THAN,
        _ => Duration::ZERO,
    }
}

pub(super) fn still_resting(
    estimated_rtt: Option<Duration>,
    last_asked: Option<f64>,
    now: f64,
) -> bool {
    let rest = reask_no_sooner_than(estimated_rtt);
    last_asked.is_some_and(|last| now - last < rest.as_secs_f64())
}

pub(super) fn forgotten_after(between_asks: Duration) -> Duration {
    (between_asks * ROUNDS_MISSED_BEFORE_FORGOTTEN).max(NEVER_FORGET_SOONER_THAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::Coordinates;

    #[test]
    fn a_node_that_has_just_started_asks_often_enough_to_settle_within_minutes() {
        let fresh = Coordinates::default().how_wrong_we_are();

        let rounds_to_settle = 40;
        let settling = between_asks(fresh) * rounds_to_settle;

        assert!(
            settling < Duration::from_secs(20 * 60),
            "settling would take {settling:?}"
        );
    }

    #[test]
    fn a_settled_node_leaves_the_mesh_alone() {
        assert_eq!(between_asks(SETTLED_BELOW / 2.0), ONCE_SETTLED);
    }

    #[test]
    fn a_peer_is_forgotten_after_the_rounds_it_missed_but_never_after_just_one() {
        assert_eq!(forgotten_after(ONCE_SETTLED), ONCE_SETTLED * 3);
        assert_eq!(forgotten_after(WHILE_SETTLING), NEVER_FORGET_SOONER_THAN);
    }

    #[test]
    fn a_fast_or_unplaced_peer_may_be_reasked_every_round() {
        assert_eq!(reask_no_sooner_than(None), Duration::ZERO);
        assert_eq!(
            reask_no_sooner_than(Some(Duration::from_millis(200))),
            Duration::ZERO
        );
    }

    #[test]
    fn a_slow_link_is_spared_the_settling_cadence() {
        assert_eq!(
            reask_no_sooner_than(Some(Duration::from_secs(2))),
            SLOW_LINKS_REASKED_NO_SOONER_THAN
        );
        assert!(SLOW_LINKS_REASKED_NO_SOONER_THAN > WHILE_SETTLING);
    }

    #[test]
    fn a_slow_peer_rests_between_asks_while_a_fast_one_never_does() {
        let slow = Some(Duration::from_secs(2));
        let floor = SLOW_LINKS_REASKED_NO_SOONER_THAN.as_secs_f64();
        assert!(still_resting(slow, Some(1_000.0), 1_000.0 + floor / 2.0));
        assert!(!still_resting(slow, Some(1_000.0), 1_000.0 + floor + 1.0));
        assert!(!still_resting(slow, None, 9_999.0));
        assert!(!still_resting(
            Some(Duration::from_millis(50)),
            Some(1_000.0),
            1_000.1
        ));
    }
}
