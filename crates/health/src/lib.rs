//! Health check registry implementation.
//!
//! See [`HealthCheckRegistry`] for details.

pub trait HealthCheck: Send + Sync + 'static {
    fn status(&self) -> eyre::Result<()>;
}

pub struct HealthCheckRegistry {
    checks: Vec<(&'static str, Box<dyn HealthCheck>)>,
}

///  The result of the health check, which can be one of the following:
/// HealthCheckResult::Ok(oks): If all health checks pass successfully. oks is a vector containing the names of the passed health checks.
/// HealthCheckResult::Fail(fails): If all health checks fail. fails is a vector containing the names of the failed health checks.
/// HealthCheckResult::Warning(oks, fails): If some health checks pass while others fail. oks is a vector containing the names of the passed health checks, and fails is a vector containing the names of the failed health checks.
pub enum HealthStatus {
    Ok(Vec<&'static str>),
    Warning(Vec<&'static str>, Vec<&'static str>),
    Fail(Vec<&'static str>),
}
/// A HealthCheckRegistry is a collection of health checks that can be registered and executed.
/// Each health check is associated with a name and is expected to implement the HealthCheck trait.
impl HealthCheckRegistry {
    pub fn new() -> Self {
        HealthCheckRegistry { checks: Vec::new() }
    }

    pub fn register(&mut self, name: &'static str, check: impl HealthCheck) {
        self.checks.push((name, Box::new(check)));
    }

    pub fn status(&self) -> HealthStatus {
        let mut fails = Vec::new();
        let mut oks = Vec::new();

        for (name, check) in &self.checks {
            match check.status() {
                Ok(_) => oks.push(*name),
                Err(_) => {
                    fails.push(*name);
                }
            }
        }

        if fails.is_empty() {
            HealthStatus::Ok(oks)
        } else if oks.is_empty() {
            HealthStatus::Fail(fails)
        } else {
            HealthStatus::Warning(oks, fails)
        }
    }
}

impl Default for HealthCheckRegistry {
    fn default() -> Self {
        HealthCheckRegistry::new()
    }
}
