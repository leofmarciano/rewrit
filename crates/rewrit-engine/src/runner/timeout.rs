use std::time::Duration;

#[must_use]
pub fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

