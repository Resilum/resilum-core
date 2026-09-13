use std::future::Future;
use std::sync::{Mutex, MutexGuard};

use tokio::task::JoinSet;

use crate::watched::reporting;

#[derive(Default)]
pub struct Nursery(Mutex<JoinSet<()>>);

impl Nursery {
    pub fn keep<F>(&self, name: impl Into<String>, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let mut kept = self.lock();
        while kept.try_join_next().is_some() {}
        kept.spawn(reporting(name, task));
    }

    pub async fn everyone_home(&self) {
        let mut kept = std::mem::take(&mut *self.lock());
        kept.shutdown().await;
    }

    fn lock(&self) -> MutexGuard<'_, JoinSet<()>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
