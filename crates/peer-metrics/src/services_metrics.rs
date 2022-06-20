use std::collections::HashMap;
use std::{fmt, time};

use async_std::task;
use futures::channel::mpsc::unbounded;
use futures::select;
use futures::stream::StreamExt;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{linear_buckets, Histogram};
use prometheus_client::registry::Registry;

use fluence_app_service::{MemoryStats, ModuleDescriptor};
use fluence_libp2p::types::{Inlet, Outlet};

use crate::{execution_time_buckets, mem_buckets_4gib, mem_buckets_8gib, register};

type ServiceId = String;
type ModuleName = String;
type MemorySize = u64;

pub struct ServiceMemoryMsg {
    service_id: String,
    memory_stat: ServiceMemoryStat,
}

#[derive(Default)]
struct ServiceMemoryStat {
    /// Memory used by the service
    used_mem: MemorySize,
    /// Memory used by the modules that belongs to the service
    modules_stats: HashMap<ModuleName, MemorySize>,
}

impl ServiceMemoryStat {
    fn new(stats: MemoryStats) -> ServiceMemoryStat {
        let mut modules_stats = HashMap::new();
        let mut used_mem: MemorySize = 0;
        for stat in stats.0 {
            modules_stats.insert(stat.name.to_string(), stat.memory_size as MemorySize);
            used_mem += stat.memory_size as MemorySize;
        }
        ServiceMemoryStat {
            used_mem,
            modules_stats,
        }
    }
}

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

    /// Number of (srv create) failures
    pub creation_failure_count: Counter,

    /// How many modules a service includes.
    pub modules_in_services_count: Histogram,

    /// Memory metrics
    pub memory_metrics: ServicesMemoryMetrics,

    //
    memory_update_outlet: Outlet<ServiceMemoryMsg>,
}

impl fmt::Debug for ServicesMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServicesMetrics").finish()
    }
}

impl ServicesMetrics {
    pub fn new(registry: &mut Registry, memory_update_outlet: Outlet<ServiceMemoryMsg>) -> Self {
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

        ServicesMetrics {
            services_count,
            creation_time_msec,
            removal_time_msec,
            creation_count,
            removal_count,
            creation_failure_count,
            modules_in_services_count,
            memory_metrics,
            memory_update_outlet,
        }
    }

    pub fn observe_service_mem(&self, service_id: String, stats: MemoryStats) {
        let msg = ServiceMemoryMsg {
            service_id,
            memory_stat: ServiceMemoryStat::new(stats),
        };
        let result = self.memory_update_outlet.unbounded_send(msg);
        if let Err(e) = result {
            log::warn!("Can't save current memory state: {:?}", e);
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

    /// Collect all metrics that are relevant on service creation.
    pub fn observe_created(&self, service_id: ServiceId, stats: MemoryStats) {
        self.services_count.inc();
        self.modules_in_services_count.observe(stats.0.len() as f64);
        self.observe_service_mem(service_id, stats);
    }
}

pub struct ServicesMetricsBackend {
    timer_resolution: time::Duration,
    metrics: ServicesMemoryMetrics,
    inlet: Inlet<ServiceMemoryMsg>,
    /// Used memory per services
    services_memory_state: HashMap<ServiceId, ServiceMemoryStat>,
}

impl ServicesMetricsBackend {
    pub fn init_service_metrics(
        timer_resolution: time::Duration,
        registry: &mut Registry,
    ) -> (Self, ServicesMetrics) {
        let (outlet, inlet) = unbounded();
        let metrics = ServicesMetrics::new(registry, outlet);
        let backend = Self::new(timer_resolution, metrics.memory_metrics.clone(), inlet);
        (backend, metrics)
    }

    pub fn new(
        timer_resolution: time::Duration,
        metrics: ServicesMemoryMetrics,
        inlet: Inlet<ServiceMemoryMsg>,
    ) -> Self {
        Self {
            timer_resolution,
            metrics,
            inlet,
            services_memory_state: HashMap::new(),
        }
    }

    pub fn start(self) -> task::JoinHandle<()> {
        task::spawn(async move {
            let mut inlet = self.inlet.fuse();
            let mut timer = async_std::stream::interval(self.timer_resolution).fuse();
            let mut services_memory_state = self.services_memory_state;
            let metrics = self.metrics;
            loop {
                select! {
                    stat = inlet.select_next_some() => {
                        Self::observe_service_mem(&mut services_memory_state, stat.service_id, stat.memory_stat);
                        // save data to the map
                    },
                    _ = timer.select_next_some() => {
                        // send data to prometheus
                        Self::store_service_mem(&metrics, &services_memory_state);
                    }
                }
            }
        })
    }

    /// Collect the current service memory metrics including metrics of the modules
    /// that belongs to the service.
    fn observe_service_mem(
        state: &mut HashMap<ServiceId, ServiceMemoryStat>,
        service_id: String,
        stats: ServiceMemoryStat,
    ) {
        state.insert(service_id, stats);
    }

    /// Actually send all collected memory metrics to Prometheus.
    fn store_service_mem(
        metrics: &ServicesMemoryMetrics,
        state: &HashMap<ServiceId, ServiceMemoryStat>,
    ) {
        // Total memory used by all the services of the node.
        let mut total = 0;
        for (_, service_stat) in state.iter() {
            metrics.mem_used_bytes.observe(service_stat.used_mem as f64);
            for stat in &service_stat.modules_stats {
                metrics.mem_used_per_module_bytes.observe(*stat.1 as f64)
            }
            total += service_stat.used_mem;
        }
        metrics.mem_used_total_bytes.set(total);
    }
}
