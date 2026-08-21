use violin::{Config, Coord, Node};

use super::{Adjustments, Space, claimed};

pub(super) const SCATTERED_WITHIN: f64 = 0.01;
const JUST_ENOUGH_LAST_MILE_TO_GROW_FROM: f64 = 10e-6;

pub(super) fn knowing_nothing_of_where_we_are() -> Node<Space, Adjustments> {
    let scattered: Node<Space, Adjustments> = Node::rand();
    let raw = scattered.coordinate().raw_coord().as_ref();
    let nearby = Coord::<Space>::from([
        raw[0] * SCATTERED_WITHIN,
        raw[1] * SCATTERED_WITHIN,
        raw[2] * SCATTERED_WITHIN,
    ]);
    let mut ours = Node::with_coord_and_cfg(nearby, with_a_last_mile());
    ours.set_error_estimate(claimed::MOST_ERROR);
    ours
}

pub(super) fn without_a_runaway_last_mile(node: &Node<Space, Adjustments>) -> Coord<Space> {
    let held = node.coordinate();
    let raw = held.raw_coord().as_ref();
    let mut trimmed = Coord::<Space>::from([raw[0], raw[1], raw[2]]);
    trimmed.set_height(held.height().min(claimed::MOST_LAST_MILE));
    trimmed.set_error_estimate(held.error_estimate());
    trimmed
}

fn with_a_last_mile() -> Config {
    Config {
        height_min: JUST_ENOUGH_LAST_MILE_TO_GROW_FROM,
        error_max: f64::MAX,
        ..Config::default()
    }
}
