use std::future::Future;
use std::panic::AssertUnwindSafe;

use futures::FutureExt as _;
use tokio::task::JoinHandle;

use crate::ending;

pub struct Watched {
    name: String,
    handle: JoinHandle<()>,
}

impl Watched {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn has_stopped(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn abort(&self) {
        self.handle.abort();
    }

    pub async fn come_home(self) {
        if self.handle.await.is_err() {
            tracing::debug!(task = %self.name, "cancelled");
        }
    }
}

pub fn watch<F>(name: impl Into<String>, run: F) -> Watched
where
    F: Future<Output = ()> + Send + 'static,
{
    let name = name.into();
    Watched {
        handle: tokio::spawn(reporting(name.clone(), run)),
        name,
    }
}

pub fn watch_on<F>(runtime: &tokio::runtime::Handle, name: impl Into<String>, run: F) -> Watched
where
    F: Future<Output = ()> + Send + 'static,
{
    let name = name.into();
    Watched {
        handle: runtime.spawn(reporting(name.clone(), run)),
        name,
    }
}

pub(crate) async fn reporting<F>(name: String, run: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    ending::report(&name, AssertUnwindSafe(run).catch_unwind().await);
}

#[cfg(test)]
mod tests;
