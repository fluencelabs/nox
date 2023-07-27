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
#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // A mock health check implementation for testing purposes.
    struct MockHealthCheck {
        should_pass: bool,
    }

    impl HealthCheck for MockHealthCheck {
        fn status(&self) -> eyre::Result<()> {
            if self.should_pass {
                Ok(())
            } else {
                Err(eyre::eyre!("Health check failed"))
            }
        }
    }

    #[test]
    fn test_health_check_registry_empty() {
        let registry = HealthCheckRegistry::new();
        let status = registry.status();
        assert_eq!(status, HealthStatus::Ok(vec![]))
    }

    #[test]
    fn test_health_check_registry_single_check_pass() {
        let mut registry = HealthCheckRegistry::new();
        let mock_check = MockHealthCheck { should_pass: true };
        registry.register("MockCheck1", mock_check);

        let status = registry.status();
        assert_eq!(status, HealthStatus::Ok(vec!["MockCheck1"]))
    }

    #[test]
    fn test_health_check_registry_single_check_fail() {
        let mut registry = HealthCheckRegistry::new();
        let mock_check = MockHealthCheck { should_pass: false };
        registry.register("MockCheck1", mock_check);

        let status = registry.status();
        assert_eq!(status, HealthStatus::Fail(vec!["MockCheck1"]))
    }

    #[test]
    fn test_health_check_registry_multiple_checks() {
        let mut registry = HealthCheckRegistry::new();
        let mock_check1 = MockHealthCheck { should_pass: true };
        let mock_check2 = MockHealthCheck { should_pass: false };
        let mock_check3 = MockHealthCheck { should_pass: true };
        registry.register("MockCheck1", mock_check1);
        registry.register("MockCheck2", mock_check2);
        registry.register("MockCheck3", mock_check3);

        let status = registry.status();
        assert_eq!(
            status,
            HealthStatus::Warning(vec!["MockCheck1", "MockCheck3"], vec!["MockCheck2"])
        );
    }
}
