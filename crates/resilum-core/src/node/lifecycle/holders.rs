use std::sync::Arc;

use crate::error::{Error, Result};

pub(super) fn only_ours<T>(held: Arc<T>) -> Result<T> {
    Arc::try_unwrap(held).map_err(|still_shared| {
        Error::Engine(format!(
            "{} holders of the engine outlived stop; its ports stay bound",
            Arc::strong_count(&still_shared)
        ))
    })
}

#[cfg(test)]
mod tests;
