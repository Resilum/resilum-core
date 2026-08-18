//! Supervised async tasks with exponential backoff: a failing task restarts
//! without taking the node down.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

/// Restart delays after consecutive failures.
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

const HEALTHY_AFTER: Duration = Duration::from_secs(30);

async fn supervise(task: Task) {
    let mut fails = 0;
    loop {
        let started = Instant::now();
        match tokio::spawn((task.run)()).await {
            Ok(()) => tracing::info!(task = %task.name, "run ended"),
            Err(join) if join.is_cancelled() => return,
            Err(join) => {
                let reason = panic_reason(join.into_panic());
                tracing::error!(task = %task.name, reason, "run panicked");
            }
        }
        if started.elapsed() >= HEALTHY_AFTER {
            fails = 0;
        }
        let delay = backoff(fails);
        fails += 1;
        tracing::warn!(task = %task.name, ?delay, "restarting");
        tokio::time::sleep(delay).await;
    }
}

/// The default panic hook writes to stderr under a worker thread's name, which
/// says nothing about which component died.
fn panic_reason(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_owned();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "a panic payload that is neither &str nor String".to_owned()
}

/// Spawn every task under supervision.
pub fn spawn_all(tasks: Vec<Task>) -> Vec<JoinHandle<()>> {
    tasks
        .into_iter()
        .map(|t| tokio::spawn(supervise(t)))
        .collect()
}

#[cfg(test)]
mod tests;
