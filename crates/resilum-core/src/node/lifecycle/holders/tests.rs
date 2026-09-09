use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Runtime;

use super::wait_until_only_ours;
use crate::Error;

fn a_runtime() -> Runtime {
    Runtime::new().expect("runtime")
}

#[test]
fn a_holder_still_unwinding_is_waited_for() {
    let runtime = a_runtime();
    let held = Arc::new(7u8);
    let leaving = Arc::clone(&held);
    runtime.spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        drop(leaving);
    });

    assert_eq!(wait_until_only_ours(&runtime, held).ok(), Some(7));
}

#[test]
fn a_holder_that_never_leaves_is_reported_rather_than_waited_for_forever() {
    let runtime = a_runtime();
    let held = Arc::new(7u8);
    let _stays = Arc::clone(&held);

    assert!(matches!(
        wait_until_only_ours(&runtime, held),
        Err(Error::Engine(_))
    ));
}
