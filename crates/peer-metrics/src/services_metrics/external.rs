/**
 * Services metrics that are meant to be written to an external metrics storage like Prometheus
 */
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{linear_buckets, Histogram};
use prometheus_client::registry::Registry;

use fluence_app_service::ModuleDescriptor;

use crate::{execution_time_buckets, mem_buckets_4gib, mem_buckets_8gib, register};

#[derive(Clone)]
pub struct ServicesMemoryMetrics {
    /// Maximum memory set in module config
    pub mem_max_bytes: Histogram,
    /// Actual memory used by a module
    pub mem_max_per_module_bytes: Histogram,
    /// Actual memory used by a service
    pub mem_used_bytes: Histogram,
    /// Actual memory used by a module
    pub mem_used_per_module_bytes: Histogram,
    /// Total memory used
    pub mem_used_total_bytes: Gauge,
}

impl ServicesMemoryMetrics {
    /// Collect the service and the service's modules  max available memory.
    pub fn observe_service_max_mem(&self, default_max: u64, modules_config: &[ModuleDescriptor]) {
        let mut max_service_size = 0;
        for module_config in modules_config {
            let module_max = module_config.config.max_heap_size.unwrap_or(default_max);
            self.mem_max_per_module_bytes.observe(module_max as f64);
            max_service_size += module_max;
        }
        self.mem_max_bytes.observe(max_service_size as f64);
    }
}

#[derive(Clone)]
pub struct ServicesMetricsExternal {
    /// Number of currently running services
    pub services_count: Gauge,
    /// How long it took to create a service
    pub creation_time_msec: Histogram,
    /// How long it took to remove a service
    pub removal_time_msec: Histogram,
    /// Number of (srv create) calls
    pub creation_count: Counter,
    /// Number of (srv remove) calls
    pub removal_count: Counter,

    /// Number of (srv create) failures
    pub creation_failure_count: Counter,

    /// How many modules a service includes.
    pub modules_in_services_count: Histogram,

    /// Memory metrics
    pub memory_metrics: ServicesMemoryMetrics,
}

impl ServicesMetricsExternal {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("services");

        let services_count = register(
            sub_registry,
            Gauge::default(),
            "services_count",
            "number of currently running services",
        );

        let creation_time_msec = register(
            sub_registry,
            Histogram::new(execution_time_buckets()),
            "creation_time_msec",
            "how long it took to create a service",
        );

        let removal_time_msec = register(
            sub_registry,
            Histogram::new(execution_time_buckets()),
            "removal_time_msec",
            "how long it took to remove a service",
        );

        let creation_count = register(
            sub_registry,
            Counter::default(),
            "creation_count",
            "number of srv create calls",
        );

        let removal_count = register(
            sub_registry,
            Counter::default(),
            "removal_count",
            "number of srv remove calls",
        );

        let mem_max_bytes = register(
            sub_registry,
            Histogram::new(mem_buckets_8gib()),
            "mem_max_bytes",
            "maximum memory set in module config per service",
        );

        let mem_max_per_module_bytes = register(
            sub_registry,
            Histogram::new(mem_buckets_4gib()),
            "mem_max_per_module_bytes",
            "maximum memory set in module config",
        );

        let mem_used_bytes = register(
            sub_registry,
            Histogram::new(mem_buckets_8gib()),
            "mem_used_bytes",
            "actual memory used by a service",
        );

        let mem_used_per_module_bytes = register(
            sub_registry,
            Histogram::new(mem_buckets_4gib()),
            "mem_used_per_module_bytes",
            "actual memory used by a service per module",
        );

        let mem_used_total_bytes = register(
            sub_registry,
            Gauge::default(),
            "mem_used_total_bytes",
            "total size of used memory by services",
        );

        let creation_failure_count = register(
            sub_registry,
            Counter::default(),
            "creation_failure_count",
            "number of srv remove calls",
        );

        let modules_in_services_count = register(
            sub_registry,
            Histogram::new(linear_buckets(1.0, 1.0, 10)),
            "modules_in_services_count",
            "number of modules per services",
        );

        let memory_metrics = ServicesMemoryMetrics {
            mem_max_bytes,
            mem_max_per_module_bytes,
            mem_used_bytes,
            mem_used_per_module_bytes,
            mem_used_total_bytes,
        };

        Self {
            services_count,
            creation_time_msec,
            removal_time_msec,
            creation_count,
            removal_count,
            creation_failure_count,
            modules_in_services_count,
            memory_metrics,
        }
    }

    /// Collect the service and the service's modules  max available memory.
    pub fn observe_service_max_mem(
        &self,
        default_max: u64,
        modules_config: &Vec<ModuleDescriptor>,
    ) {
        self.memory_metrics
            .observe_service_max_mem(default_max, modules_config);
    }

    /// Collect all metrics that are relevant on service removal.
    pub fn observe_removed(&self, removal_time: f64) {
        self.removal_count.inc();
        self.services_count.dec();
        self.removal_time_msec.observe(removal_time);
    }
}
