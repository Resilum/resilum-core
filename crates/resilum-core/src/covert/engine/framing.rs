//! Sliding-window ARQ over a lossy datagram carrier.

use std::time::Duration;

pub use self::recv::RecvBuffer;
pub use self::send::SendBuffer;

mod recv;
mod send;

pub const RTO_INITIAL: Duration = Duration::from_secs(2);
pub const RTO_MIN: Duration = Duration::from_millis(500);
pub const RTO_MAX: Duration = Duration::from_secs(30);

fn clamp_rto(rto: Duration) -> Duration {
    rto.max(RTO_MIN).min(RTO_MAX)
}

#[cfg(test)]
mod tests;
