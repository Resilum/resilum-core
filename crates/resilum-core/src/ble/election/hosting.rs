use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Default)]
pub struct HostingTheGroup(Arc<AtomicBool>);

impl HostingTheGroup {
    #[must_use]
    pub fn nobody_yet() -> Self {
        Self::default()
    }

    pub fn raise(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn stand_down(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_up(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
