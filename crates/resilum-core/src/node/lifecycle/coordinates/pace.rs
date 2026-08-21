use std::time::Duration;

const WHILE_SETTLING: Duration = Duration::from_secs(20);
const ONCE_SETTLED: Duration = Duration::from_secs(900);
const SETTLED_BELOW: f64 = 0.25;
const ROUNDS_MISSED_BEFORE_FORGOTTEN: u32 = 3;
const NEVER_FORGET_SOONER_THAN: Duration = Duration::from_secs(600);

pub(super) fn between_asks(our_error: f64) -> Duration {
    if our_error > SETTLED_BELOW {
        WHILE_SETTLING
    } else {
        ONCE_SETTLED
    }
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
}
