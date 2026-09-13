use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

#[tokio::test]
async fn aborting_the_watcher_takes_the_task_with_it() {
    let still_running = Arc::new(AtomicBool::new(false));
    let watched = still_running.clone();

    let watcher = watch("a task that never ends on its own", async move {
        loop {
            watched.store(true, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });
    while !still_running.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    watcher.abort();
    watcher.come_home().await;
    still_running.store(false, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    assert!(
        !still_running.load(Ordering::SeqCst),
        "the task went on running after its watcher was aborted"
    );
}

#[tokio::test]
async fn a_panicking_task_is_reported_and_leaves_its_watcher_standing() {
    let watcher = watch("a task that panics", async {
        panic!("the reason it died");
    });

    watcher.come_home().await;
}

#[tokio::test]
async fn a_task_that_ended_says_so() {
    let watched = watch("a task that ends at once", async {});
    while !watched.has_stopped() {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    assert_eq!(watched.name(), "a task that ends at once");
}
