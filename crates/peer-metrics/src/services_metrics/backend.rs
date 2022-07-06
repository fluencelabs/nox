use std::collections::HashMap;
use std::time;

use async_std::task;
use futures::select;
use futures::stream::StreamExt;

use fluence_libp2p::types::Inlet;

use crate::services_metrics::builtin::ServicesMetricsBuiltin;
use crate::services_metrics::instant::ServicesMemoryMetrics;
use crate::services_metrics::message::{ServiceMemoryStat, ServiceMetricsMsg};

type ServiceId = String;

/// Data that is used for instant metrics when they're enabled.
struct InstantBackend {
    /// How often to send memory data to prometheus
    timer_resolution: time::Duration,
    /// Collection of prometheus handlers
    memory_metrics: ServicesMemoryMetrics,
    /// Used memory per services
    services_memory_state: HashMap<ServiceId, ServiceMemoryStat>,
}

/// The backend creates a separate threads that processes
/// requests from critical sections of code (where we can't afford to wait on locks)
/// to store some metrics.
pub struct ServicesMetricsBackend {
    inlet: Inlet<ServiceMetricsMsg>,
    instant_data: Option<InstantBackend>,
    builtin_metrics: ServicesMetricsBuiltin,
}

impl ServicesMetricsBackend {
    /// Create fully a functional backend for both instant and builtin metrics.
    pub fn with_instant_metrics(
        timer_resolution: time::Duration,
        memory_metrics: ServicesMemoryMetrics,
        builtin_metrics: ServicesMetricsBuiltin,
        inlet: Inlet<ServiceMetricsMsg>,
    ) -> Self {
        let instant_data = InstantBackend {
            timer_resolution,
            memory_metrics,
            services_memory_state: HashMap::new(),
        };
        Self {
            inlet,
            instant_data: Some(instant_data),
            builtin_metrics,
        }
    }

    /// Create a backend with only builtin metrics gathering enabled.
    pub fn new(builtin_metrics: ServicesMetricsBuiltin, inlet: Inlet<ServiceMetricsMsg>) -> Self {
        Self {
            inlet,
            instant_data: None,
            builtin_metrics,
        }
    }

    pub fn start(self) -> task::JoinHandle<()> {
        if let Some(instant) = self.instant_data {
            Self::start_with_instant(self.inlet, self.builtin_metrics, instant)
        } else {
            Self::start_builtin_only(self.inlet, self.builtin_metrics)
        }
    }

    fn start_with_instant(
        inlet: Inlet<ServiceMetricsMsg>,
        builtin_metrics: ServicesMetricsBuiltin,
        instant_data: InstantBackend,
    ) -> task::JoinHandle<()> {
        task::spawn(async move {
            let mut inlet = inlet.fuse();
            let mut timer = async_std::stream::interval(instant_data.timer_resolution).fuse();
            let mut services_memory_state = instant_data.services_memory_state;
            let memory_metrics = instant_data.memory_metrics;
            loop {
                select! {
                    msg = inlet.select_next_some() => {
                        match msg {
                            // save data to the map
                            ServiceMetricsMsg::Memory { service_id, memory_stat } => {
                                Self::observe_service_mem(&mut services_memory_state, service_id, memory_stat);
                            },
                            ServiceMetricsMsg::CallStats { service_id, function_name, stats } => {
                                builtin_metrics.update(service_id, function_name, stats);
                            },
                        }
                    },
                    _ = timer.select_next_some() => {
                        // send data to prometheus
                        Self::store_service_mem(&memory_metrics, &services_memory_state);
                    }
                }
            }
        })
    }

    fn start_builtin_only(
        inlet: Inlet<ServiceMetricsMsg>,
        builtin_metrics: ServicesMetricsBuiltin,
    ) -> task::JoinHandle<()> {
        task::spawn(async move {
            let mut inlet = inlet.fuse();
            loop {
                select! {
                    msg = inlet.select_next_some() => {
                        match msg {
                            // save data to the map
                            ServiceMetricsMsg::Memory{..} => {
                                log::warn!("Can't collect memory metrics: instant metrics are disabled.");
                            },
                            ServiceMetricsMsg::CallStats { service_id, function_name, stats } => {
                                builtin_metrics.update(service_id, function_name, stats);
                            },
                        }
                    },
                }
            }
        })
    }

    /// Collect the current service memory metrics including memory metrics of the modules
    /// that belongs to the service.
    fn observe_service_mem(
        state: &mut HashMap<ServiceId, ServiceMemoryStat>,
        service_id: String,
        stats: ServiceMemoryStat,
    ) {
        state.insert(service_id, stats);
    }

    /// Actually send all collected memory memory_metrics to Prometheus.
    fn store_service_mem(
        memory_metrics: &ServicesMemoryMetrics,
        state: &HashMap<ServiceId, ServiceMemoryStat>,
    ) {
        // Total memory used by all the services of the node.
        let mut total = 0;
        for (_, service_stat) in state.iter() {
            memory_metrics
                .mem_used_bytes
                .observe(service_stat.used_mem as f64);
            for stat in &service_stat.modules_stats {
                memory_metrics
                    .mem_used_per_module_bytes
                    .observe(*stat.1 as f64)
            }
            total += service_stat.used_mem;
        }
        memory_metrics.mem_used_total_bytes.set(total);
    }
}
