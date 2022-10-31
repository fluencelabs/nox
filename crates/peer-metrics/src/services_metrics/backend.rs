use std::collections::HashMap;
use std::time;

use async_std::task;
use futures::select;
use futures::stream::StreamExt;

use fluence_libp2p::types::Inlet;

use crate::services_metrics::builtin::ServicesMetricsBuiltin;
use crate::services_metrics::external::{ServiceTypeLabel, ServicesMemoryMetrics};
use crate::services_metrics::message::{ServiceMemoryStat, ServiceMetricsMsg};
use crate::ServiceType;

type ServiceId = String;

/// Metrics that are meant to be written to an external metrics storage like Prometheus
struct ExternalMetricsBackend {
    /// How often to send memory data to prometheus
    timer_resolution: time::Duration,
    /// Collection of prometheus handlers
    memory_metrics: ServicesMemoryMetrics,
    /// Used memory per services
    services_memory_stats: HashMap<ServiceId, (ServiceType, ServiceMemoryStat)>,
}

/// The backend creates a separate threads that processes
/// requests from critical sections of code (where we can't afford to wait on locks)
/// to store some metrics.
pub struct ServicesMetricsBackend {
    inlet: Inlet<ServiceMetricsMsg>,
    external_metrics: Option<ExternalMetricsBackend>,
    builtin_metrics: ServicesMetricsBuiltin,
}

impl ServicesMetricsBackend {
    /// Create fully a functional backend for both external and builtin metrics.
    pub fn with_external_metrics(
        timer_resolution: time::Duration,
        memory_metrics: ServicesMemoryMetrics,
        builtin_metrics: ServicesMetricsBuiltin,
        inlet: Inlet<ServiceMetricsMsg>,
    ) -> Self {
        let external_metrics = ExternalMetricsBackend {
            timer_resolution,
            memory_metrics,
            services_memory_stats: HashMap::new(),
        };
        Self {
            inlet,
            external_metrics: Some(external_metrics),
            builtin_metrics,
        }
    }

    /// Create a backend with only builtin metrics gathering enabled.
    pub fn new(builtin_metrics: ServicesMetricsBuiltin, inlet: Inlet<ServiceMetricsMsg>) -> Self {
        Self {
            inlet,
            external_metrics: None,
            builtin_metrics,
        }
    }

    pub fn start(self) -> task::JoinHandle<()> {
        if let Some(external_metrics) = self.external_metrics {
            Self::start_with_external(self.inlet, self.builtin_metrics, external_metrics)
        } else {
            Self::start_builtin_only(self.inlet, self.builtin_metrics)
        }
    }

    fn start_with_external(
        inlet: Inlet<ServiceMetricsMsg>,
        builtin_metrics: ServicesMetricsBuiltin,
        external_metrics: ExternalMetricsBackend,
    ) -> task::JoinHandle<()> {
        task::spawn(async move {
            let mut inlet = inlet.fuse();
            let mut timer = async_std::stream::interval(external_metrics.timer_resolution).fuse();
            let mut services_memory_stats = external_metrics.services_memory_stats;
            let memory_metrics = external_metrics.memory_metrics;
            loop {
                select! {
                    msg = inlet.select_next_some() => {
                        match msg {
                            // save data to the map
                            ServiceMetricsMsg::Memory { service_id, service_type, memory_stat } => {
                                Self::observe_service_mem(&mut services_memory_stats, service_id, service_type, memory_stat);
                            },
                            ServiceMetricsMsg::CallStats { service_id, function_name, stats } => {
                                builtin_metrics.update(service_id, function_name, stats);
                            },
                        }
                    },
                    _ = timer.select_next_some() => {
                        // send data to prometheus
                        Self::store_service_mem(&memory_metrics, &services_memory_stats);
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
                            ServiceMetricsMsg::Memory{..} => {},
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
        all_stats: &mut HashMap<ServiceId, (ServiceType, ServiceMemoryStat)>,
        service_id: String,
        service_type: ServiceType,
        service_stat: ServiceMemoryStat,
    ) {
        all_stats.insert(service_id, (service_type, service_stat));
    }

    /// Actually send all collected memory memory_metrics to Prometheus.
    fn store_service_mem(
        memory_metrics: &ServicesMemoryMetrics,
        all_stats: &HashMap<ServiceId, (ServiceType, ServiceMemoryStat)>,
    ) {
        let mut unaliased_service_total_memory = 0;
        for (_, (service_type, service_stat)) in all_stats.iter() {
            let service_type_label = ServiceTypeLabel {
                service_type: service_type.clone(),
            };
            memory_metrics
                .mem_used_bytes
                .get_or_create(&service_type_label)
                .observe(service_stat.used_mem as f64);
            for stat in &service_stat.modules_stats {
                memory_metrics
                    .mem_used_per_module_bytes
                    .get_or_create(&service_type_label)
                    .observe(*stat.1 as f64)
            }

            if matches!(service_type, ServiceType::Service(Some(_))) {
                memory_metrics
                    .mem_used_total_bytes
                    .get_or_create(&service_type_label)
                    .set(service_stat.used_mem);
            } else {
                unaliased_service_total_memory += service_stat.used_mem
            }
        }

        memory_metrics
            .mem_used_total_bytes
            .get_or_create(&ServiceTypeLabel {
                service_type: ServiceType::Service(None),
            })
            .set(unaliased_service_total_memory);
    }
}
