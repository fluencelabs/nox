use open_metrics_client::encoding::text::Encode;
use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::family::Family;
use open_metrics_client::metrics::gauge::Gauge;
use open_metrics_client::metrics::histogram::Histogram;
use open_metrics_client::registry::Registry;

#[derive(Encode, Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum ServiceCall {
    Builtin,
    Service,
}

#[derive(Clone)]
pub struct ParticleExecutorMetrics {
    pub interpretation_time_sec: Histogram,
    pub normalized_interpretation_time_sec: Histogram,
    pub interpretation_successes: Counter,
    pub interpretation_failures: Counter,
    pub free_interpreters: Gauge,
    pub total_actors_mailbox: Gauge,
    pub alive_actors: Gauge,
    pub service_call_time_sec: Family<ServiceCall, Histogram>,
    pub service_call_success: Family<ServiceCall, Counter>,
    pub service_call_failure: Family<ServiceCall, Counter>,
}

impl ParticleExecutorMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("particle_executor");

        let interpretation_time_sec = Histogram::new(execution_time_buckets().into_iter());
        sub_registry.register(
            "interpretation_time_sec",
            "Distribution of time it took to run the interpreter once",
            Box::new(interpretation_time_sec.clone()),
        );

        let normalized_interpretation_time_sec =
            Histogram::new(execution_time_buckets().into_iter());
        sub_registry.register(
            "normalized_interpretation_time_sec",
            "Distribution of interpreter run time divided by resulting particle.data size",
            Box::new(normalized_interpretation_time_sec.clone()),
        );

        let interpretation_successes = Counter::default();
        sub_registry.register(
            "interpretation_successes",
            "Number successfully interpreted particles",
            Box::new(interpretation_successes.clone()),
        );

        let interpretation_failures = Counter::default();
        sub_registry.register(
            "interpretation_failures",
            "Number of failed particle interpretations",
            Box::new(interpretation_failures.clone()),
        );

        let free_interpreters = Gauge::default();
        sub_registry.register(
            "free_interpreters",
            "Number of currently free AquaVMs",
            Box::new(free_interpreters.clone()),
        );
        let total_actors_mailbox = Gauge::default();
        sub_registry.register(
            "total_actors_mailbox",
            "Cumulative sum of all actors' mailboxes",
            Box::new(total_actors_mailbox.clone()),
        );
        let alive_actors = Gauge::default();
        sub_registry.register(
            "alive_actors",
            "Number of currently alive actors (1 particle id = 1 actor)",
            Box::new(alive_actors.clone()),
        );

        let service_call_time_sec: Family<_, _> =
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets().into_iter()));
        sub_registry.register(
            "service_call_time_sec",
            "Distribution of time it took to execute a single service or builtin call",
            Box::new(service_call_time_sec.clone()),
        );
        let service_call_success = Family::default();
        sub_registry.register(
            "service_call_success",
            "Number of succeeded service calls",
            Box::new(service_call_success.clone()),
        );
        let service_call_failure = Family::default();
        sub_registry.register(
            "service_call_failure",
            "Number of failed service calls",
            Box::new(service_call_failure.clone()),
        );

        Self {
            interpretation_time_sec,
            normalized_interpretation_time_sec,
            interpretation_successes,
            interpretation_failures,
            free_interpreters,
            total_actors_mailbox,
            alive_actors,
            service_call_time_sec,
            service_call_success,
            service_call_failure,
        }
    }
}

/// from 100 microseconds to 30 seconds
fn execution_time_buckets() -> Vec<f64> {
    vec![
        0.0001, 0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 7.0, 15.0, 30.0,
    ]
}
