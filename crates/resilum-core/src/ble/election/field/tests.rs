use super::Field;
use crate::ble::election::{Facts, the_one_to_host};
use crate::ble::links::PeerId;

const US: PeerId = [0; 16];
const THEM: PeerId = [9; 16];

fn able() -> Facts {
    Facts {
        has_an_uplink: true,
        charging: true,
        battery_percent: 100,
        ..Facts::default()
    }
}

#[test]
fn a_node_we_never_met_over_the_radio_cannot_talk_its_way_into_the_field() {
    let field = Field::default();

    field.told_us(THEM, able(), true);

    assert!(field.whom_we_hear().is_empty());
    assert_eq!(
        field
            .standing_with_us(US, Facts::default(), true)
            .iter()
            .map(|candidate| candidate.who)
            .collect::<Vec<_>>(),
        vec![US]
    );
}

#[test]
fn a_peer_met_but_not_yet_heard_from_stands_without_being_able_to_host() {
    let field = Field::default();

    field.met_over_the_radio(THEM, 0);

    let standing = field.standing_with_us(US, Facts::default(), false);
    assert_eq!(standing.len(), 2);
    assert_eq!(the_one_to_host(&standing), None);
}

#[test]
fn what_a_peer_we_met_says_of_itself_is_what_it_stands_on() {
    let field = Field::default();
    field.met_over_the_radio(THEM, 0);

    field.told_us(THEM, able(), true);

    let standing = field.standing_with_us(US, Facts::default(), true);
    assert_eq!(
        the_one_to_host(&standing).map(|winner| winner.who),
        Some(THEM)
    );
}

#[test]
fn a_peer_that_parts_stops_standing() {
    let field = Field::default();
    field.met_over_the_radio(THEM, 0);
    field.told_us(THEM, able(), true);

    field.gone(THEM);

    assert_eq!(field.how_many_we_hear(), 0);
    assert_eq!(
        the_one_to_host(&field.standing_with_us(US, Facts::default(), true)).map(|w| w.who),
        Some(US)
    );
}

#[test]
fn a_neighbour_that_has_said_nothing_is_worth_hearing_out_only_so_long() {
    let field = Field::default();
    field.met_over_the_radio(THEM, 1_000);

    assert!(field.someone_met_is_still_worth_hearing_out(2_000, 5_000));
    assert!(!field.someone_met_is_still_worth_hearing_out(9_000, 5_000));
}

#[test]
fn a_neighbour_that_has_spoken_is_not_waited_for_again() {
    let field = Field::default();
    field.met_over_the_radio(THEM, 0);

    field.told_us(THEM, able(), true);

    assert!(!field.someone_met_is_still_worth_hearing_out(0, 5_000));
}

#[test]
fn we_stand_in_our_own_field_with_nobody_else_around() {
    let field = Field::default();

    let standing = field.standing_with_us(US, able(), true);

    assert_eq!(the_one_to_host(&standing).map(|w| w.who), Some(US));
}
