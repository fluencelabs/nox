/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::HashMap;
use std::time;

use futures::stream::StreamExt;
use tokio::select;
use tokio::sync::mpsc;
use tokio::task::{Builder, JoinHandle};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

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
    inlet: mpsc::UnboundedReceiver<ServiceMetricsMsg>,
    external_metrics: Option<ExternalMetricsBackend>,
    builtin_metrics: ServicesMetricsBuiltin,
}

impl ServicesMetricsBackend {
    /// Create fully a functional backend for both external and builtin metrics.
    pub fn with_external_metrics(
        timer_resolution: time::Duration,
        memory_metrics: ServicesMemoryMetrics,
        builtin_metrics: ServicesMetricsBuiltin,
        inlet: mpsc::UnboundedReceiver<ServiceMetricsMsg>,
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
    pub fn new(
        builtin_metrics: ServicesMetricsBuiltin,
        inlet: mpsc::UnboundedReceiver<ServiceMetricsMsg>,
    ) -> Self {
        Self {
            inlet,
            external_metrics: None,
            builtin_metrics,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        if let Some(external_metrics) = self.external_metrics {
            Self::start_with_external(self.inlet, self.builtin_metrics, external_metrics)
        } else {
            Self::start_builtin_only(self.inlet, self.builtin_metrics)
        }
    }

    fn start_with_external(
        mut inlet: mpsc::UnboundedReceiver<ServiceMetricsMsg>,
        builtin_metrics: ServicesMetricsBuiltin,
        external_metrics: ExternalMetricsBackend,
    ) -> JoinHandle<()> {
        Builder::new().name("Metrics").spawn(async move {
            let mut timer = IntervalStream::new(interval(external_metrics.timer_resolution));
            let mut services_memory_stats = external_metrics.services_memory_stats;
            let memory_metrics = external_metrics.memory_metrics;
            loop {
                select! {
                    Some(msg) = inlet.recv() => {
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
                    _ = timer.next() => {
                        // send data to prometheus
                        Self::store_service_mem(&memory_metrics, &services_memory_stats);
                    }
                }
            }
        }).expect("Could not spawn task")
    }

    fn start_builtin_only(
        mut inlet: mpsc::UnboundedReceiver<ServiceMetricsMsg>,
        builtin_metrics: ServicesMetricsBuiltin,
    ) -> JoinHandle<()> {
        Builder::new().name("Metrics").spawn(async move {
            loop {
                select! {
                    Some(msg) = inlet.recv() => {
                        match msg {
                            ServiceMetricsMsg::Memory{..} => {},
                            ServiceMetricsMsg::CallStats { service_id, function_name, stats } => {
                                builtin_metrics.update(service_id, function_name, stats);
                            },
                        }
                    },
                }
            }
        }).expect("Could not spawn task")
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
        let mut unaliased_spells_total_memory = 0;
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
            match service_type {
                ServiceType::Service(Some(_)) | ServiceType::Spell(Some(_)) => {
                    let used_mem = i64::try_from(service_stat.used_mem);
                    match used_mem {
                        Ok(used_mem) => {
                            memory_metrics
                                .mem_used_total_bytes
                                .get_or_create(&service_type_label)
                                .set(used_mem);
                        }
                        Err(e) => log::warn!("Could not convert metric used_mem {}", e),
                    }
                }
                ServiceType::Spell(_) => {
                    unaliased_spells_total_memory += service_stat.used_mem;
                }
                _ => {
                    unaliased_service_total_memory += service_stat.used_mem;
                }
            }
        }

        match i64::try_from(unaliased_service_total_memory) {
            Ok(unaliased_service_total_memory) => {
                memory_metrics
                    .mem_used_total_bytes
                    .get_or_create(&ServiceTypeLabel {
                        service_type: ServiceType::Service(None),
                    })
                    .set(unaliased_service_total_memory);
            }
            _ => log::warn!("Could not convert metric unaliased_service_total_memory"),
        }

        match i64::try_from(unaliased_spells_total_memory) {
            Ok(unaliased_spells_total_memory) => {
                memory_metrics
                    .mem_used_total_bytes
                    .get_or_create(&ServiceTypeLabel {
                        service_type: ServiceType::Spell(None),
                    })
                    .set(unaliased_spells_total_memory);
            }
            _ => log::warn!("Could not convert metric unaliased_service_total_memory"),
        }
    }
}
