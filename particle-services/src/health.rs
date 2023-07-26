use health::HealthCheck;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PersistedServiceHealth {
    started: Arc<RwLock<bool>>,
    has_errors: Arc<RwLock<bool>>,
}

impl PersistedServiceHealth {
    pub fn new() -> Self {
        PersistedServiceHealth {
            started: Arc::new(RwLock::new(false)),
            has_errors: Arc::new(RwLock::new(false)),
        }
    }

    pub fn start_creation(&mut self) {
        let mut guard = self.started.write();
        *guard = true;
    }

    pub fn finish_creation(&mut self) {
        let mut guard = self.has_errors.write();
        *guard = true;
    }
}

impl HealthCheck for PersistedServiceHealth {
    fn status(&self) -> eyre::Result<()> {
        let started_guard = self.started.read();
        let errors_guard = self.has_errors.read();
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
