use super::{A_MESH_OF, a_full_set_nearest_first, met_over_the_radio, peer};
use crate::discovery::quota::{Verdict, held_for_the_radio_of, judge, kept_of};

#[test]
fn a_neighbour_within_radio_range_gets_in_where_a_distant_peer_would_not() {
    let full = a_full_set_nearest_first();

    assert_eq!(
        judge(&full, met_over_the_radio(99), A_MESH_OF),
        Verdict::Attach
    );
    assert_eq!(judge(&full, peer(99, None), A_MESH_OF), Verdict::Refuse);
}

#[test]
fn the_share_held_for_the_radio_does_not_grow_without_bound() {
    let room = kept_of(A_MESH_OF);
    let mut full = a_full_set_nearest_first();
    full.extend((0..held_for_the_radio_of(room) as u8).map(|nth| met_over_the_radio(nth + 100)));

    assert_eq!(
        judge(&full, met_over_the_radio(99), A_MESH_OF),
        Verdict::Refuse
    );
}
