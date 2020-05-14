#![cfg(test)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn current_time() -> Duration {
    Duration::from_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    )
}
