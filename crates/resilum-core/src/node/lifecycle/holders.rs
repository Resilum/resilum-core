use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::runtime::Runtime;

use crate::error::{Error, Result};

const A_HOLDER_ON_ITS_WAY_OUT_IS_GIVEN: Duration = Duration::from_secs(2);
const BETWEEN_TRIES: Duration = Duration::from_millis(5);

pub(super) fn wait_until_only_ours<T>(runtime: &Runtime, held: Arc<T>) -> Result<T> {
    let give_up_at = Instant::now() + A_HOLDER_ON_ITS_WAY_OUT_IS_GIVEN;
    let mut held = held;
    loop {
        match Arc::try_unwrap(held) {
            Ok(ours_alone) => return Ok(ours_alone),
            Err(still_shared) if Instant::now() < give_up_at => {
                held = still_shared;
                runtime.block_on(async { tokio::time::sleep(BETWEEN_TRIES).await });
            }
            Err(still_shared) => {
                return Err(Error::Engine(format!(
                    "{} holders of the engine outlived stop; its ports stay bound",
                    Arc::strong_count(&still_shared)
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests;
