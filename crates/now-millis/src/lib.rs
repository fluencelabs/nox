use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Returns UNIX timestamp as Duration
pub fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
}

/// Returns UNIX timestamp in milliseconds
pub fn now_ms() -> u128 {
    now().as_millis()
}

/// Returns UNIX timestamp in seconds
pub fn now_sec() -> u64 {
    now().as_secs()
}
