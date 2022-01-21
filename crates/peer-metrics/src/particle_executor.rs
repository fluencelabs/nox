use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::histogram::Histogram;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct ParticleExecutorMetrics {
    interpretation_time: Histogram,
    normalized_interpretation_time: Histogram,
    service_call_time: Histogram,
    execution_successes: Counter,
    execution_failures: Counter,
}

impl ParticleExecutorMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("particle_executor");

        let interpretation_time = Histogram::default();
        sub_registry.register(
            "interpretation_time",
            "Distribution of time it took to run the interpreter once",
            Box::new(interpretation_time.clone()),
        );

        let normalized_interpretation_time = Histogram::default();
        sub_registry.register(
            "normalized_interpretation_time",
            "Distribution of interpreter run time divided by resulting particle.data size",
            Box::new(normalized_interpretation_time.clone()),
        );

        let service_call_time = Histogram::default();
        sub_registry.register(
            "service_call_time",
            "Distribution of time it took to execute a single service or builtin call",
            Box::new(service_call_time.clone()),
        );

        let execution_successes = Counter::default();
        sub_registry.register(
            "execution_successes",
            "Number successfully executed particles",
            Box::new(execution_successes.clone()),
        );

        let execution_failures = Counter::default();
        sub_registry.register(
            "execution_failures",
            "Number of failed particle executions",
            Box::new(execution_failures.clone()),
        );

        Self {
            interpretation_time,
            normalized_interpretation_time,
            service_call_time,
            execution_successes,
            execution_failures,
        }
    }
}
