//! The propagation node read off `LxmfHandle::propagation_node`: the same
//! value the status snapshot serialises for a caller.

use super::harness::{Harness, STAMP_COST};

#[test]
fn a_selected_propagation_node_is_published_to_the_handle() {
    let harness = Harness::new("selected", STAMP_COST);

    harness.publish();

    assert_eq!(
        harness.handle.propagation_node(),
        Some(*harness.propagation_node.as_bytes()),
    );
}

/// The one mutation that tells a following value from a latched one: publish
/// once with a node selected, clear the selection, publish again, and the
/// second read has to have moved — a value set once at start and never
/// touched again would still read `Some` here.
#[test]
fn clearing_the_selection_turns_the_published_node_back_to_none() {
    let mut harness = Harness::new("cleared", STAMP_COST);
    harness.publish();
    assert!(
        harness.handle.propagation_node().is_some(),
        "setup did not select one"
    );

    let _cleared = harness
        .ready
        .router
        .set_outbound_propagation_node(&mut harness.core, None)
        .expect("clear the selection");
    harness.publish();

    assert_eq!(harness.handle.propagation_node(), None);
}
