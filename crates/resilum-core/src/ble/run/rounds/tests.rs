use super::{Field, LOOK_AROUND_BEFORE_JUDGING_MS, the_field_is_not_known_yet};
use crate::ble::election::Facts;
use crate::ble::spec::HEAR_A_NEIGHBOUR_OUT_MS;

const A_NEIGHBOUR: [u8; 16] = [3; 16];

#[test]
fn nothing_is_judged_before_the_radio_has_had_time_to_find_anyone() {
    let empty = Field::default();

    assert!(the_field_is_not_known_yet(
        &empty,
        LOOK_AROUND_BEFORE_JUDGING_MS - 1
    ));
    assert!(!the_field_is_not_known_yet(
        &empty,
        LOOK_AROUND_BEFORE_JUDGING_MS
    ));
}

#[test]
fn a_neighbour_met_late_is_heard_out_before_anything_is_decided() {
    let field = Field::default();
    let met_at = LOOK_AROUND_BEFORE_JUDGING_MS * 2;
    field.met_over_the_radio(A_NEIGHBOUR, met_at);

    assert!(the_field_is_not_known_yet(&field, met_at + 1));

    field.told_us(A_NEIGHBOUR, Facts::default(), true);

    assert!(!the_field_is_not_known_yet(&field, met_at + 1));
}

#[test]
fn a_neighbour_that_never_answers_stops_holding_the_decision_up() {
    let field = Field::default();
    field.met_over_the_radio(A_NEIGHBOUR, 0);

    assert!(!the_field_is_not_known_yet(&field, HEAR_A_NEIGHBOUR_OUT_MS));
}
