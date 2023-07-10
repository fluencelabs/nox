pub trait HealthCheck: Send + Sync + 'static {
    fn check(&self) -> eyre::Result<()>;
}

pub struct HealthCheckRegistry {
    checks: Vec<(String, Box<dyn HealthCheck>)>,
}

pub enum HealthCheckResult {
    Ok(Vec<String>),
    Warning(Vec<String>, Vec<String>),
    Fail(Vec<String>),
}

impl HealthCheckRegistry {
    pub fn new() -> Self {
        HealthCheckRegistry { checks: Vec::new() }
    }

    pub fn register(&mut self, name: &str, check: impl HealthCheck) {
        self.checks.push((name.to_string(), Box::new(check)));
    }

    pub fn check(&self) -> HealthCheckResult {
        let mut fails = Vec::new();
        let mut oks = Vec::new();

        for (name, check) in &self.checks {
            match check.check() {
                Ok(_) => oks.push(name.clone()),
                Err(_) => {
                    fails.push(name.clone());
                }
            }
        }

        if fails.is_empty() {
            HealthCheckResult::Ok(oks)
        } else if fails.len() == self.checks.len() {
            HealthCheckResult::Fail(fails)
        } else {
            HealthCheckResult::Warning(oks, fails)
        }
    }
}

impl Default for HealthCheckRegistry {
    fn default() -> Self {
        HealthCheckRegistry::new()
    }
}
