use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{linear_buckets, Histogram};
use prometheus_client::registry::Registry;

use fluence_app_service::MemoryStats;

use crate::{execution_time_buckets, mem_buckets_4gib, mem_buckets_8gib, registered};

type ServiceId = String;
type ModuleName = String;
type MemorySize = u64;

struct ServiceMemoryStat {
    total: MemorySize,
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

        let services_count = registered(
            sub_registry,
            Gauge::default,
            "serivces_count",
            "number of currently running services",
        );

        let creation_time_msec = registered(
            sub_registry,
            || Histogram::new(execution_time_buckets()),
            "creation_time_msec",
            "how long it took to create a service",
        );

        let removal_time_msec = registered(
            sub_registry,
            || Histogram::new(execution_time_buckets()),
            "removal_time_msec",
            "how long it took to remove a service",
        );

        let creation_count = registered(
            sub_registry,
            Counter::default,
            "creation_count",
            "number of srv create calls",
        );

        let removal_count = registered(
            sub_registry,
            Counter::default,
            "removal_count",
            "number of srv remove calls",
        );

        let mem_max_bytes = registered(
            sub_registry,
            || Histogram::new(mem_buckets_8gib()),
            "mem_max_bytes",
            "maximum memory set in module config per service",
        );

        let mem_max_per_module_bytes = registered(
            sub_registry,
            || Histogram::new(mem_buckets_4gib()),
            "mem_max_per_module_bytes",
            "maximum memory set in module config",
        );

        let mem_used_bytes = registered(
            sub_registry,
            || Histogram::new(mem_buckets_8gib()),
            "mem_used_bytes",
            "actual memory used by a service",
        );

        let mem_used_per_module_bytes = registered(
            sub_registry,
            || Histogram::new(mem_buckets_4gib()),
            "mem_used_per_module_bytes",
            "actual memory used by a service per module",
        );

        let mem_used_total_bytes = registered(
            sub_registry,
            Gauge::default,
            "mem_used_total_bytes",
            "total size of used memory by services",
        );

        let creation_failure_count = registered(
            sub_registry,
            Counter::default,
            "creation_failure_count",
            "number of srv remove calls",
        );

        let modules_in_services_count = registered(
            sub_registry,
            || Histogram::new(linear_buckets(1.0, 1.0, 10)),
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

    pub fn observe_service_mem(&self, service_id: String, stats: MemoryStats) {
        let mut state = self.services_memory_state.lock().unwrap();

        // Update or create a service stats including corresponding modules' stats.
        if let Some(service_stat) = state.get_mut(&service_id) {
            // Count total used memory by a service and update services' modules' memory stats.
            let mut total: MemorySize = 0;
            for module_stat in stats.0 {
                if let Some(current_size) = service_stat.modules_stats.get_mut(module_stat.name) {
                    *current_size = module_stat.memory_size as MemorySize;
                    total += *current_size;
                }
            }
            service_stat.total = total;
        } else {
            let total = stats.0.iter().fold(0, |acc, x| acc + x.memory_size) as MemorySize;
            let modules_stats = stats
                .0
                .into_iter()
                .map(|stat| (stat.name.to_string(), stat.memory_size as MemorySize))
                .collect::<_>();
            state.insert(
                service_id,
                ServiceMemoryStat {
                    total,
                    modules_stats,
                },
            );
        }
    }

    pub fn store_service_mem(&self) {
        let state = self.services_memory_state.lock().unwrap();
        let mut total = 0;
        for (_, service_stat) in state.iter() {
            self.mem_used_bytes.observe(service_stat.total as f64);
            for stat in &service_stat.modules_stats {
                self.mem_used_per_module_bytes.observe(*stat.1 as f64)
            }
            total += service_stat.total;
        }
        self.mem_used_total_bytes.set(total);
    }

    pub fn observe_service_max_mem(&self, memory: &[u64]) {
        let mut size = 0;
        for m in memory {
            self.mem_max_per_module_bytes.observe(*m as f64);
            size += m;
        }
        self.mem_max_bytes.observe(size as f64);
    }
}
