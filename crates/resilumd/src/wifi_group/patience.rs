use std::time::Duration;

const A_GROUP_IS_GIVEN_THIS_LONG_TO_FILL: Duration = Duration::from_secs(90);
const BEFORE_THE_FIRST_RETRY: Duration = Duration::from_secs(60);
const NO_LONGER_THAN: Duration = Duration::from_secs(1_800);

#[must_use]
pub(super) fn it_carried_nobody(carries: usize, since_raised: Duration) -> bool {
    carries == 0 && since_raised >= A_GROUP_IS_GIVEN_THIS_LONG_TO_FILL
}

#[must_use]
pub(super) fn before_trying_again(turned_away: u32) -> Duration {
    BEFORE_THE_FIRST_RETRY
        .saturating_mul(2u32.saturating_pow(turned_away.saturating_sub(1).min(15)))
        .min(NO_LONGER_THAN)
}

#[cfg(test)]
mod tests;
