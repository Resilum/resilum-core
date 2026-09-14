use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::time::Duration;

use futures::FutureExt as _;
use tokio::time::Instant;

use crate::ending;
use crate::watched::Watched;

const BACKOFF_SECS: [u64; 6] = [1, 2, 5, 15, 30, 60];

const HEALTHY_AFTER: Duration = Duration::from_secs(30);

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
        let started = Instant::now();
        match AssertUnwindSafe((task.run)()).catch_unwind().await {
            Ok(()) => tracing::info!(task = %task.name, "run ended"),
            Err(payload) => {
                let reason = ending::reason(payload);
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

pub fn keep_alive(tasks: Vec<Task>) -> Vec<Watched> {
    tasks
        .into_iter()
        .map(|t| {
            let named = t.name.clone();
            crate::watch(named, supervise(t))
        })
        .collect()
}

#[cfg(test)]
mod tests;
