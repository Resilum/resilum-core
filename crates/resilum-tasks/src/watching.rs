use std::sync::{Mutex, MutexGuard};

use crate::watched::Watched;

#[derive(Default)]
pub struct Watching(Mutex<Vec<Watched>>);

impl Watching {
    pub fn keep(&self, task: Watched) {
        self.lock().push(task);
    }

    pub fn keep_all(&self, tasks: impl IntoIterator<Item = Watched>) {
        self.lock().extend(tasks);
    }

    #[must_use]
    pub fn whichever_stopped(&self) -> Vec<String> {
        self.lock()
            .iter()
            .filter(|task| task.has_stopped())
            .map(|task| task.name().to_owned())
            .collect()
    }

    pub async fn everyone_home(&self) {
        let kept = std::mem::take(&mut *self.lock());
        for task in &kept {
            task.abort();
        }
        for task in kept {
            task.come_home().await;
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Watched>> {
        self.0.lock().unwrap_or_else(|held| held.into_inner())
    }
}

#[cfg(test)]
mod tests;
