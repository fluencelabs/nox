use health::HealthCheck;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PersistedServiceHealthCheck {
    started: Arc<Mutex<bool>>,
    has_errors: Arc<Mutex<bool>>,
}

impl PersistedServiceHealthCheck {
    pub fn new() -> Self {
        PersistedServiceHealthCheck {
            started: Arc::new(Mutex::new(false)),
            has_errors: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start_loading(&mut self) {
        let mut guard = self.started.lock();
        *guard = true;
    }

    pub fn mark_has_errors(&mut self) {
        let mut guard = self.has_errors.lock();
        *guard = true;
    }
}

impl HealthCheck for PersistedServiceHealthCheck {
    fn check(&self) -> eyre::Result<()> {
        let started_guard = self.started.lock();
        let errors_guard = self.has_errors.lock();
        let started = *started_guard;
        if started {
            let has_errors = *errors_guard;
            if has_errors {
                Err(eyre::eyre!("Persisted services loading failed"))
            } else {
                Ok(())
            }
        } else {
            Err(eyre::eyre!("Not loaded yet"))
        }
    }
}
