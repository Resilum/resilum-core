use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::time::{Duration, Instant, sleep};

use super::{BACKOFF_SECS, HEALTHY_AFTER, Task, backoff, panic_reason, supervise};

const HELD_FOR: Duration = Duration::from_secs(HEALTHY_AFTER.as_secs() * 2);

#[test]
fn backoff_follows_schedule_and_saturates() {
    assert_eq!(backoff(0).as_secs(), 1);
    assert_eq!(backoff(3).as_secs(), 15);
    assert_eq!(backoff(99).as_secs(), *BACKOFF_SECS.last().unwrap());
}

#[test]
fn a_panic_reason_survives_whichever_way_it_was_raised() {
    let literal = std::panic::catch_unwind(|| panic!("a literal")).expect_err("panicked");
    assert_eq!(panic_reason(literal), "a literal");

    let formatted = std::panic::catch_unwind(|| panic!("formatted {}", 7)).expect_err("panicked");
    assert_eq!(panic_reason(formatted), "formatted 7");

    let odd = std::panic::catch_unwind(|| std::panic::panic_any(7u8)).expect_err("panicked");
    assert!(panic_reason(odd).contains("neither"));
}

#[tokio::test(start_paused = true)]
async fn a_run_that_stayed_up_clears_the_backoff() {
    let starts: Arc<Mutex<Vec<Instant>>> = Arc::default();
    let runs = Arc::new(AtomicUsize::new(0));

    let (seen, count) = (Arc::clone(&starts), Arc::clone(&runs));
    let task = Task::new("probe", move || {
        let (seen, count) = (Arc::clone(&seen), Arc::clone(&count));
        async move {
            seen.lock().expect("starts").push(Instant::now());
            if count.fetch_add(1, Ordering::SeqCst) == 2 {
                sleep(HELD_FOR).await;
            }
        }
    });

    let supervising = tokio::spawn(supervise(task));
    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline && starts.lock().expect("starts").len() < 4 {
        sleep(Duration::from_secs(1)).await;
    }
    supervising.abort();

    let seen = starts.lock().expect("starts").clone();
    assert!(seen.len() >= 4, "the loop did not get four runs in");
    assert_eq!((seen[1] - seen[0]).as_secs(), backoff(0).as_secs());
    assert_eq!((seen[2] - seen[1]).as_secs(), backoff(1).as_secs());
    assert_eq!(
        (seen[3] - seen[2]).as_secs(),
        HELD_FOR.as_secs() + backoff(0).as_secs()
    );
}
