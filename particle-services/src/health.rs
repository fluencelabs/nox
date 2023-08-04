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
            has_errors: Arc::new(RwLock::new(true)), // this is true by default because we wont to show error while loading in progress
        }
    }

    pub fn start_creation(&mut self) {
        let mut guard = self.started.write();
        *guard = true;
    }

    pub fn finish_creation(&mut self) {
        let mut guard = self.has_errors.write();
        *guard = false;
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
            Err(eyre::eyre!(
                "Persisted services creation hasn't started yet"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_persisted_service_health_not_started() {
        let health = PersistedServiceHealth::new();
        let status = health.status();
        assert!(status.is_err());
    }

    #[test]
    fn test_persisted_service_health_started_with_errors() {
        let mut health = PersistedServiceHealth::new();
        health.start_creation();
        let status = health.status();
        assert!(status.is_err());
    }

    #[test]
    fn test_persisted_service_health_started_without_errors() {
        let mut health = PersistedServiceHealth::new();
        health.start_creation();
        health.finish_creation();
        let status = health.status();
        assert!(status.is_ok());
    }

    #[test]
    fn persisted_service_health_concurrent_access() {
        let persisted_health = Arc::new(RwLock::new(PersistedServiceHealth::new()));
        let health_clone = persisted_health.clone();

        let thread_handle = thread::spawn(move || {
            let mut health = health_clone.write();
            health.start_creation();
            health.finish_creation();
        });

        thread_handle.join().unwrap();

        let status = persisted_health.read().status();
        assert!(status.is_ok());
    }
}
