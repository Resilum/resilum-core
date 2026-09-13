use super::*;
use crate::watch;

#[tokio::test]
async fn a_task_that_ended_is_named_and_a_running_one_is_not() {
    let watching = Watching::default();
    watching.keep(watch("one that ends", async {}));
    watching.keep(watch("one that runs on", std::future::pending()));

    while watching.whichever_stopped().is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    assert_eq!(
        watching.whichever_stopped(),
        vec!["one that ends".to_owned()]
    );
}

#[tokio::test]
async fn sending_everyone_home_leaves_nothing_behind() {
    let watching = Watching::default();
    watching.keep(watch("one that runs on", std::future::pending()));

    watching.everyone_home().await;

    assert!(watching.whichever_stopped().is_empty());
}
