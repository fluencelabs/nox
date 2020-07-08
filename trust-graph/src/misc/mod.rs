use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn current_time() -> Duration {
    Duration::from_secs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64,
    )
}
