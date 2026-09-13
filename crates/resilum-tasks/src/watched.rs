use std::future::Future;

use tokio::task::{AbortHandle, JoinHandle};

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
        ending::report(&self.name, self.handle.await);
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
    let inner = tokio::spawn(run);
    let taken_along_if_we_are_aborted = TakenAlong(inner.abort_handle());
    ending::report(&name, inner.await);
    drop(taken_along_if_we_are_aborted);
}

struct TakenAlong(AbortHandle);

impl Drop for TakenAlong {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests;
