use health::HealthCheck;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct VMPoolHealth {
    expected_count: usize,
    current_count: Arc<AtomicUsize>,
}

impl VMPoolHealth {
    pub fn new(expected_count: usize) -> Self {
        Self {
            expected_count,
            current_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_count(&self) {
        self.current_count.fetch_add(1, Ordering::Release);
    }
}

impl HealthCheck for VMPoolHealth {
    fn status(&self) -> eyre::Result<()> {
        let current = self.current_count.load(Ordering::Acquire);
        if self.expected_count != current {
            return Err(eyre::eyre!(
                "VM pool isn't full. Current: {}, Expected: {}",
                current,
                self.expected_count
            ));
        }

        Ok(())
    }
}
