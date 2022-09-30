use prometheus_client::encoding::text::Encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use std::time::Duration;

use crate::execution_time_buckets;

#[derive(Copy, Clone, Debug, Encode, Hash, Eq, PartialEq)]
pub enum FunctionKind {
    Builtin,
    Service,
    Function,
    // Function call failed early
    NotHappened,
}

#[derive(Encode, Hash, Clone, Eq, PartialEq)]
pub struct FunctionKindLabel {
    function_kind: FunctionKind,
}

#[derive(Clone)]
pub struct ParticleExecutorMetrics {
    pub interpretation_time_sec: Histogram,
    pub interpretation_successes: Counter,
    pub interpretation_failures: Counter,
    pub total_actors_mailbox: Gauge,
    pub alive_actors: Gauge,
    service_call_time_sec: Family<FunctionKindLabel, Histogram>,
    service_call_success: Family<FunctionKindLabel, Counter>,
    service_call_failure: Family<FunctionKindLabel, Counter>,
}

impl ParticleExecutorMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("particle_executor");

        let interpretation_time_sec = Histogram::new(execution_time_buckets());
        sub_registry.register(
            "interpretation_time_sec",
            "Distribution of time it took to run the interpreter once",
            Box::new(interpretation_time_sec.clone()),
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
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets()));
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
            interpretation_successes,
            interpretation_failures,
            total_actors_mailbox,
            alive_actors,
            service_call_time_sec,
            service_call_success,
            service_call_failure,
        }
    }

    pub fn service_call(&self, success: bool, kind: FunctionKind, run_time: Option<Duration>) {
        let label = FunctionKindLabel {
            function_kind: kind,
        };

        if success {
            self.service_call_success.get_or_create(&label).inc();
        } else {
            self.service_call_failure.get_or_create(&label).inc();
        }
        if let Some(run_time) = run_time {
            self.service_call_time_sec
                .get_or_create(&label)
                .observe(run_time.as_secs_f64())
        }
    }
}
