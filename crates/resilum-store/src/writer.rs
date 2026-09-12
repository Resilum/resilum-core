use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

pub struct Writer<C> {
    changes: Option<Sender<C>>,
    thread: Option<JoinHandle<()>>,
}

impl<C: Send + 'static> Writer<C> {
    pub fn spawn<H: Send + 'static>(
        path: PathBuf,
        mut held: H,
        apply: fn(&mut H, C),
        write: fn(&Path, &H),
    ) -> Self {
        let (changes, arriving) = std::sync::mpsc::channel::<C>();
        let thread = std::thread::spawn(move || {
            while let Ok(change) = arriving.recv() {
                apply(&mut held, change);
                while let Ok(still_arriving) = arriving.try_recv() {
                    apply(&mut held, still_arriving);
                }
                write(&path, &held);
            }
        });
        Self {
            changes: Some(changes),
            thread: Some(thread),
        }
    }

    #[must_use]
    pub fn nowhere_to_write() -> Self {
        Self {
            changes: None,
            thread: None,
        }
    }

    pub fn send(&self, change: C) {
        let Some(changes) = &self.changes else { return };
        if changes.send(change).is_err() {
            tracing::error!("the writer thread is gone; changes no longer reach the disk");
        }
    }
}

impl<C> Drop for Writer<C> {
    fn drop(&mut self) {
        let closing_the_channel_is_what_ends_the_thread = self.changes.take();
        drop(closing_the_channel_is_what_ends_the_thread);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("the writer thread panicked; its last changes are not on disk");
        }
    }
}
