use super::{MAX_BACKOFF, MIN_BACKOFF, grow};

#[test]
fn backoff_doubles_from_min_backoff_and_stops_at_max_backoff() {
    let mut backoff = MIN_BACKOFF;
    let mut grown = Vec::new();
    for _ in 0..7 {
        backoff = grow(backoff);
        grown.push(backoff);
    }

    assert_eq!(
        grown,
        [2, 4, 8, 16, 32, 60, 60].map(std::time::Duration::from_secs)
    );
    assert_eq!(grow(MAX_BACKOFF), MAX_BACKOFF);
}
