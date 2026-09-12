use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(all(unix, feature = "ygg"))]
pub(crate) fn say_the_address_is(path: &std::path::Path, address: &str) -> std::io::Result<()> {
    resilum_store::write_text(path, address)
}

pub(crate) struct Advertised {
    path: Option<PathBuf>,
    known: Mutex<Option<String>>,
}

impl Advertised {
    pub(crate) fn at(path: Option<PathBuf>) -> Self {
        Self {
            path,
            known: Mutex::new(None),
        }
    }

    pub(crate) fn is_served_from_a_file(&self) -> bool {
        self.path.is_some()
    }

    pub(crate) fn read(&self) -> Option<String> {
        let mut known = self.lock();
        if known.is_some() {
            return known.clone();
        }
        let path = self.path.as_ref()?;
        let raw = resilum_store::read_text(path).ok()?;
        let host = raw.trim();
        *known = (!host.is_empty()).then(|| host.to_owned());
        known.clone()
    }

    pub(crate) fn forget(&self) {
        *self.lock() = None;
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if let Err(e) = resilum_store::forget(path) {
            tracing::warn!(path = %path.display(), error = %e, "the advertised address outlives the service that served it");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.known.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
#[path = "hostname_tests.rs"]
mod tests;
