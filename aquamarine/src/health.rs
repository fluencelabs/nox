use health::HealthCheck;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct VMPoolHealth {
    expected_count: usize,
    current_count: Arc<Mutex<usize>>,
}

impl VMPoolHealth {
    pub fn new(expected_count: usize) -> Self {
        Self {
            expected_count,
            current_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn increment_count(&self) {
        let mut guard = self.current_count.lock();
        *guard += 1;
    }
}

impl HealthCheck for VMPoolHealth {
    fn status(&self) -> eyre::Result<()> {
        let guard = self.current_count.lock();
        let current = *guard;
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
