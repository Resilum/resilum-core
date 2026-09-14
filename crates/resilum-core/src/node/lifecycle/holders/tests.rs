use std::sync::Arc;

use super::only_ours;
use crate::Error;

#[test]
fn the_last_holder_gets_what_it_held() {
    assert_eq!(only_ours(Arc::new(7u8)).ok(), Some(7));
}

#[test]
fn a_holder_that_outlived_stop_is_reported_with_how_many_there_are() {
    let held = Arc::new(7u8);
    let _stays = Arc::clone(&held);

    let Err(Error::Engine(said)) = only_ours(held) else {
        panic!("a shared engine must not be handed out as ours alone");
    };
    assert!(said.contains('2'), "{said}");
}
