use super::*;

const A_MESH_OF: usize = 4096;

fn ms(millis: u64) -> Option<Duration> {
    Some(Duration::from_millis(millis))
}

fn peer(nth: u8, estimate: Option<Duration>) -> Peer {
    Peer {
        attached_as: format!("tor[peer{nth}]:4242"),
        estimate,
    }
}

fn a_full_set_nearest_first() -> Vec<Peer> {
    (0..kept_of(A_MESH_OF) as u8)
        .map(|nth| peer(nth, ms(u64::from(nth) * 10 + 10)))
        .collect()
}

#[test]
fn a_bigger_mesh_asks_for_more_contacts_but_never_without_bound() {
    assert_eq!(kept_of(4096), 13);
    assert_eq!(kept_of(1_000_000), 20);
    assert_eq!(kept_of(0), FEWEST_KEPT);
    assert_eq!(kept_of(usize::MAX), MOST_KEPT);
}

#[test]
fn a_node_with_room_takes_whoever_it_hears() {
    assert_eq!(judge(&[], peer(1, None), A_MESH_OF), Verdict::Attach);
    assert_eq!(
        judge(
            &a_full_set_nearest_first()[..2],
            peer(1, ms(900)),
            A_MESH_OF
        ),
        Verdict::Attach
    );
}

#[test]
fn a_nearer_newcomer_displaces_the_worst_of_the_near_ones() {
    let kept = a_full_set_nearest_first();
    let room = kept_of(A_MESH_OF);

    let verdict = judge(&kept, peer(99, ms(5)), A_MESH_OF);

    let worst_near = peer((room - 1 - crossing_of(room)) as u8, None).attached_as;
    assert_eq!(verdict, Verdict::Replace(worst_near));
}

#[test]
fn a_middling_newcomer_is_refused() {
    let kept = a_full_set_nearest_first();

    let slower_than_every_near_one_and_nearer_than_every_far_one = ms(105);

    let verdict = judge(
        &kept,
        peer(99, slower_than_every_near_one_and_nearer_than_every_far_one),
        A_MESH_OF,
    );

    assert_eq!(verdict, Verdict::Refuse);
}

#[test]
fn a_newcomer_further_than_anyone_kept_gets_in_so_the_clusters_stay_joined() {
    let kept = a_full_set_nearest_first();

    let verdict = judge(&kept, peer(99, ms(9_000)), A_MESH_OF);

    assert!(matches!(verdict, Verdict::Replace(_)), "{verdict:?}");
}

#[test]
fn a_peer_this_node_cannot_place_gives_way_to_one_it_can() {
    let mut kept = a_full_set_nearest_first();
    kept[4] = peer(4, None);

    let verdict = judge(&kept, peer(99, ms(45)), A_MESH_OF);

    assert_eq!(verdict, Verdict::Replace(peer(4, None).attached_as));
}

#[test]
fn a_newcomer_nobody_can_place_does_not_displace_a_measured_peer() {
    let kept = a_full_set_nearest_first();

    assert_eq!(judge(&kept, peer(99, None), A_MESH_OF), Verdict::Refuse);
}
