use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::histogram::{exponential_buckets, Histogram};
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct ParticleExecutorMetrics {
    interpretation_time_sec: Histogram,
    normalized_interpretation_time_sec: Histogram,
    service_call_time: Histogram,
    execution_successes: Counter,
    execution_failures: Counter,
}

impl ParticleExecutorMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("particle_executor");

        // from 100 microseconds to 30 seconds
        let execution_time_buckets = vec![
            0.0001, 0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 7.0, 15.0, 30.0,
        ];
        let interpretation_time_sec = Histogram::new(execution_time_buckets.iter().copied());
        sub_registry.register(
            "interpretation_time_sec",
            "Distribution of time it took to run the interpreter once",
            Box::new(interpretation_time_sec.clone()),
        );

        let normalized_interpretation_time_sec =
            Histogram::new(execution_time_buckets.iter().copied());
        sub_registry.register(
            "normalized_interpretation_time_sec",
            "Distribution of interpreter run time divided by resulting particle.data size",
            Box::new(normalized_interpretation_time_sec.clone()),
        );

        let service_call_time = Histogram::new(execution_time_buckets.iter().copied());
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
            interpretation_time_sec,
            normalized_interpretation_time_sec,
            service_call_time,
            execution_successes,
            execution_failures,
        }
    }
}
