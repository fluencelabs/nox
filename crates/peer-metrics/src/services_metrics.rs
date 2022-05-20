use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

use crate::{execution_time_buckets, mem_buckets_extended};

type ServiceId = String;
type MemorySize = f64;

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
    /// Actual memory used by service
    pub mem_used_bytes: Histogram,
    /// Total memory used
    pub mem_used_total_bytes: Gauge,
    /// Number of (srv create) failures
    pub creation_failure_count: Counter,

    /// Used memory per services
    pub services_memory_state: Arc<Mutex<HashMap<ServiceId, MemorySize>>>,
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
        sub_registry.register(
            "services_count",
            "number of currently running services on a node",
            Box::new(services_count.clone()),
        );

        let creation_time_msec = Histogram::new(execution_time_buckets());
        sub_registry.register(
            "creation_time_msec",
            "how long it took to create a service",
            Box::new(creation_time_msec.clone()),
        );

        let removal_time_msec = Histogram::new(execution_time_buckets());
        sub_registry.register(
            "removal_time_msec",
            "how long it took to remove a service",
            Box::new(removal_time_msec.clone()),
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

        let mem_used_total_bytes = Gauge::default();
        sub_registry.register(
            "mem_used_total_bytes",
            "total size of used memory by services",
            Box::new(mem_used_total_bytes.clone()),
        );

        let creation_failure_count = Counter::default();
        sub_registry.register(
            "creation_failure_count",
            "number of srv remove calls",
            Box::new(creation_failure_count.clone()),
        );

        let services_memory_state = Arc::new(Mutex::new(HashMap::new()));

        ServicesMetrics {
            services_count,
            creation_time_msec,
            removal_time_msec,
            creation_count,
            removal_count,
            mem_max_bytes,
            mem_used_bytes,
            mem_used_total_bytes,
            creation_failure_count,
            services_memory_state,
        }
    }

    pub fn monitor_service_mem(&self, service_id: String, memory: usize) {
        let mut state = self.services_memory_state.lock().unwrap();
        let old = state.insert(service_id, memory as f64);
        self.mem_used_total_bytes
            .inc_by(memory as u64 - old.unwrap_or(0.0) as u64);
    }

    pub fn store_service_mem(&self) {
        let state = self.services_memory_state.lock().unwrap();
        for (_, metric) in state.iter() {
            self.mem_used_bytes.observe(*metric);
        }
    }
}
