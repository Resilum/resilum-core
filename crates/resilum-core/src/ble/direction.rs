use super::beacon::Beacon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    DialNow,
    LetThemDialFirst,
}

#[must_use]
pub fn toward(ours: &Beacon, theirs: Option<&Beacon>) -> Toward {
    match theirs {
        Some(theirs) if ours.tiebreak < theirs.tiebreak => Toward::DialNow,
        Some(theirs) if ours.tiebreak > theirs.tiebreak => Toward::LetThemDialFirst,
        _ => Toward::LetThemDialFirst,
    }
}

#[cfg(test)]
mod tests {
    use super::{Toward, toward};
    use crate::ble::beacon::Beacon;

    fn beacon(tiebreak: [u8; 3]) -> Beacon {
        Beacon {
            tiebreak,
            can_host: false,
            group_is_up: false,
        }
    }

    #[test]
    fn of_two_of_ours_exactly_one_dials() {
        let ours = beacon([1, 0, 0]);
        let theirs = beacon([2, 0, 0]);

        assert_eq!(toward(&ours, Some(&theirs)), Toward::DialNow);
        assert_eq!(toward(&theirs, Some(&ours)), Toward::LetThemDialFirst);
    }

    #[test]
    fn a_peer_we_cannot_sort_against_is_given_the_first_move() {
        let ours = beacon([1, 0, 0]);

        assert_eq!(toward(&ours, None), Toward::LetThemDialFirst);
    }

    #[test]
    fn a_tie_leaves_neither_dialling_before_the_hold_off() {
        let ours = beacon([7, 7, 7]);
        let same = beacon([7, 7, 7]);

        assert_eq!(toward(&ours, Some(&same)), Toward::LetThemDialFirst);
    }
}
