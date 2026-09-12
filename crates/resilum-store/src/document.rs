use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use crate::files;
use crate::writer::Writer;

pub struct Document<T> {
    held: Mutex<T>,
    writer: Writer<T>,
}

impl<T: Clone + Send + 'static> Document<T> {
    pub fn open(
        path: PathBuf,
        empty: T,
        decode: fn(&[u8]) -> Result<T, String>,
        encode: fn(&T) -> Vec<u8>,
    ) -> Result<Self, String> {
        let held = match files::read_bytes(&path) {
            Ok(bytes) => decode(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => empty,
            Err(e) => return Err(format!("{} cannot be read: {e}", path.display())),
        };
        Ok(Self::holding(path, held, encode))
    }

    pub fn open_or_start_empty(
        path: PathBuf,
        empty: T,
        decode: fn(&[u8]) -> Result<T, String>,
        encode: fn(&T) -> Vec<u8>,
    ) -> Self {
        match files::read_bytes(&path).map(|bytes| decode(&bytes)) {
            Ok(Ok(held)) => Self::holding(path, held, encode),
            _ => Self::holding(path, empty, encode),
        }
    }

    fn holding(path: PathBuf, held: T, encode: fn(&T) -> Vec<u8>) -> Self {
        Self {
            writer: Writer::spawn(path, (held.clone(), encode), replace, write_atomically),
            held: Mutex::new(held),
        }
    }

    #[must_use]
    pub fn in_memory(empty: T) -> Self {
        Self {
            held: Mutex::new(empty),
            writer: Writer::nowhere_to_write(),
        }
    }

    pub fn read<R>(&self, look: impl FnOnce(&T) -> R) -> R {
        look(&self.lock())
    }

    pub fn change<R>(&self, apply: impl FnOnce(&mut T) -> R) -> R {
        let mut held = self.lock();
        let answer = apply(&mut held);
        self.writer.send(held.clone());
        answer
    }

    fn lock(&self) -> MutexGuard<'_, T> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

type Written<T> = (T, fn(&T) -> Vec<u8>);

fn replace<T>(held: &mut Written<T>, latest: T) {
    held.0 = latest;
}

fn write_atomically<T>(path: &std::path::Path, held: &Written<T>) {
    let (latest, encode) = held;
    if let Err(e) = files::replace_with(path, &encode(latest)) {
        tracing::warn!(path = %path.display(), error = %e, "a store could not be written");
    }
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
