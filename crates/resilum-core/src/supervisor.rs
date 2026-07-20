//! Supervised async tasks with exponential backoff — the in-process analog of
//! the supervisor. A failing task restarts without taking the node down.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::task::JoinHandle;

/// Restart delays after consecutive failures, matching supervisor.
const BACKOFF_SECS: [u64; 6] = [1, 2, 5, 15, 30, 60];

fn backoff(fails: usize) -> Duration {
    Duration::from_secs(BACKOFF_SECS[fails.min(BACKOFF_SECS.len() - 1)])
}

type BoxRun = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// A component to keep alive: a name and a factory for one run of it.
pub struct Task {
    name: String,
    run: BoxRun,
}

impl Task {
    pub fn new<F, Fut>(name: impl Into<String>, run: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            name: name.into(),
            run: Box::new(move || Box::pin(run())),
        }
    }
}

async fn supervise(task: Task) {
    let mut fails = 0;
    loop {
        // A panic in one run is isolated here; cancellation stops supervision.
        if let Err(join) = tokio::spawn((task.run)()).await
            && join.is_cancelled()
        {
            return;
        }
        let delay = backoff(fails);
        fails += 1;
        tracing::warn!(task = %task.name, ?delay, "restarting");
        tokio::time::sleep(delay).await;
    }
}

/// Spawn every task under supervision.
pub fn spawn_all(tasks: Vec<Task>) -> Vec<JoinHandle<()>> {
    tasks
        .into_iter()
        .map(|t| tokio::spawn(supervise(t)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BACKOFF_SECS, backoff};

    #[test]
    fn backoff_follows_schedule_and_saturates() {
        assert_eq!(backoff(0).as_secs(), 1);
        assert_eq!(backoff(3).as_secs(), 15);
        assert_eq!(backoff(99).as_secs(), *BACKOFF_SECS.last().unwrap());
    }
}
