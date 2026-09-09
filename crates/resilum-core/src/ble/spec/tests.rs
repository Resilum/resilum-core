use super::{HEAR_A_NEIGHBOUR_OUT_MS, WE_SAY_WHERE_WE_ANSWER_EVERY};

#[test]
fn a_neighbour_hears_where_we_answer_before_it_stops_waiting_for_us() {
    assert!(
        WE_SAY_WHERE_WE_ANSWER_EVERY.as_millis() <= u128::from(HEAR_A_NEIGHBOUR_OUT_MS),
        "a candidate cannot be dialled until our destination has been announced"
    );
}
