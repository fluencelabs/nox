use std::time::{SystemTime, UNIX_EPOCH};

/// Returns UNIX timestamp in milliseconds
pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
}
