use std::fmt;

use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

use crate::{execution_time_buckets, mem_buckets_extended};

#[derive(Clone)]
pub struct ServicesMetrics {
    /// Number of currently running services
    pub services_count: Gauge,
    /// How long it took to create a service
    pub creation_time_sec: Histogram,
    /// How long it took to remove a service
    pub removal_time_sec: Histogram,
    /// Number of (srv create) calls
    pub creation_count: Counter,
    /// Number of (srv remove) calls
    pub removal_count: Counter,
    /// Maximum memory set in module config
    pub mem_max_bytes: Histogram,
    /// Actual memory used by service
    pub mem_used_bytes: Histogram,
    /// Free memory in a service (max - used)
    pub mem_free_bytes: Histogram,
    /// Number of (srv create) failures
    pub creation_failure_count: Counter,
}

impl fmt::Debug for ServicesMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServicesMetrics").finish()
    }
}

impl ServicesMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("services");

        let services_count = Gauge::default();
        //let services_count = Counter::default();
        sub_registry.register(
            "services_count",
            "number of currently running services on a node",
            Box::new(services_count.clone()),
        );

        let creation_time_sec = Histogram::new(execution_time_buckets());
        sub_registry.register(
            "creation_time_sec",
            "how long it took to create a service",
            Box::new(creation_time_sec.clone()),
        );

        let removal_time_sec = Histogram::new(execution_time_buckets());
        sub_registry.register(
            "removal_time_sec",
            "how long it took to remove a service",
            Box::new(removal_time_sec.clone()),
        );

        let creation_count = Counter::default();
        sub_registry.register(
            "creation_count",
            "number of srv create calls",
            Box::new(creation_count.clone()),
        );

        let removal_count = Counter::default();
        sub_registry.register(
            "removal_count",
            "number of srv remove calls",
            Box::new(removal_count.clone()),
        );

        let mem_max_bytes = Histogram::new(mem_buckets_extended());
        sub_registry.register(
            "mem_max_bytes",
            "maximum memory set in module config",
            Box::new(mem_max_bytes.clone()),
        );

        let mem_used_bytes = Histogram::new(mem_buckets_extended());
        sub_registry.register(
            "mem_used_bytes",
            "actual memory used by a service",
            Box::new(mem_used_bytes.clone()),
        );

        let mem_free_bytes = Histogram::new(mem_buckets_extended());
        sub_registry.register(
            "mem_free_bytes",
            "free memory of a service (max minus used)",
            Box::new(mem_free_bytes.clone()),
        );

        let creation_failure_count = Counter::default();
        sub_registry.register(
            "creation_failure_count",
            "number of srv remove calls",
            Box::new(creation_failure_count.clone()),
        );

        ServicesMetrics {
            services_count,
            creation_time_sec,
            removal_time_sec,
            creation_count,
            removal_count,
            mem_max_bytes,
            mem_used_bytes,
            mem_free_bytes,
            creation_failure_count,
        }
    }
}
