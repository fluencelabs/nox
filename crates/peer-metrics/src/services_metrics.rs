use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{fmt, time};

use async_std::task;
use futures::stream::StreamExt;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{linear_buckets, Histogram};
use prometheus_client::registry::Registry;

use fluence_app_service::{MemoryStats, ModuleDescriptor};

use crate::{execution_time_buckets, mem_buckets_4gib, mem_buckets_8gib, register};

type ServiceId = String;
type ModuleName = String;
type MemorySize = u64;

#[derive(Default)]
struct ServiceMemoryStat {
    /// Memory used by the service
    used_mem: MemorySize,
    /// Memory used by the modules that belongs to the service
    modules_stats: HashMap<ModuleName, MemorySize>,
}

#[derive(Clone)]
pub struct ServicesMetrics {
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
    /// Number of (srv create) failures
    pub creation_failure_count: Counter,

    /// How many modules a service includes.
    pub modules_in_services_count: Histogram,

    /// Used memory per services
    services_memory_state: Arc<Mutex<HashMap<ServiceId, ServiceMemoryStat>>>,
}

impl fmt::Debug for ServicesMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServicesMetrics").finish()
    }
}

impl ServicesMetrics {
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

        let services_memory_state = Arc::new(Mutex::new(HashMap::new()));

        ServicesMetrics {
            services_count,
            creation_time_msec,
            removal_time_msec,
            creation_count,
            removal_count,
            mem_max_bytes,
            mem_max_per_module_bytes,
            mem_used_bytes,
            mem_used_per_module_bytes,
            mem_used_total_bytes,
            creation_failure_count,
            modules_in_services_count,
            services_memory_state,
        }
    }

    /// Collect the current service memory metrics including metrics of the modules
    /// that belongs to the service.
    pub fn observe_service_mem(&self, service_id: String, stats: MemoryStats) {
        let mut state = self.services_memory_state.lock().unwrap();

        let service_stat = state.entry(service_id).or_default();
        // Count total used memory by a service and update services' modules' memory stats.
        let mut total: MemorySize = 0;
        for stat in stats.0 {
            let current_size = service_stat
                .modules_stats
                .entry(stat.name.to_string())
                .or_default();
            *current_size = stat.memory_size as MemorySize;
            total += *current_size;
        }
        service_stat.used_mem = total;
    }

    /// Actually send all collected memory metrics to Prometheus.
    pub(crate) fn store_service_mem(&self) {
        let state = self.services_memory_state.lock().unwrap();
        // Total memory used by all the services of the node.
        let mut total = 0;
        for (_, service_stat) in state.iter() {
            self.mem_used_bytes.observe(service_stat.used_mem as f64);
            for stat in &service_stat.modules_stats {
                self.mem_used_per_module_bytes.observe(*stat.1 as f64)
            }
            total += service_stat.used_mem;
        }
        self.mem_used_total_bytes.set(total);
    }

    /// Collect the service and the service's modules  max available memory.
    pub fn observe_service_max_mem(
        &self,
        default_max: u64,
        modules_config: &Vec<ModuleDescriptor>,
    ) {
        let mut max_service_size = 0;
        for module_config in modules_config {
            let module_max = module_config.config.max_heap_size.unwrap_or(default_max);
            self.mem_max_per_module_bytes.observe(module_max as f64);
            max_service_size += module_max;
        }
        self.mem_max_bytes.observe(max_service_size as f64);
    }

    /// Collect all metrics that are relevant on service removal.
    pub fn observe_removed(&self, removal_time: f64) {
        self.removal_count.inc();
        self.services_count.dec();
        self.removal_time_msec.observe(removal_time);
    }

    /// Collect all metrics that are relevant on service creation.
    pub fn observe_created(&self, service_id: ServiceId, stats: MemoryStats) {
        self.services_count.inc();
        self.modules_in_services_count.observe(stats.0.len() as f64);
        self.observe_service_mem(service_id, stats);
    }
}

pub struct ServicesMetricsBackend {
    timer_resolution: time::Duration,
    metrics: ServicesMetrics,
}

impl ServicesMetricsBackend {
    pub fn new(timer_resolution: time::Duration, metrics: ServicesMetrics) -> Self {
        Self {
            timer_resolution,
            metrics,
        }
    }

    pub fn start(self) -> task::JoinHandle<()> {
        task::spawn(async move {
            let mut timer = async_std::stream::interval(self.timer_resolution).fuse();
            loop {
                self.metrics.store_service_mem();
                timer.next().await;
            }
        })
    }
}
