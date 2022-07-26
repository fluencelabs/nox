pub mod backend;
pub mod builtin;
pub mod instant;
pub mod message;

use std::{fmt, time::Duration};

use prometheus_client::registry::Registry;

use futures::channel::mpsc::unbounded;

use fluence_app_service::MemoryStats;
use fluence_libp2p::types::Outlet;

pub use crate::services_metrics::backend::ServicesMetricsBackend;
pub use crate::services_metrics::builtin::ServicesMetricsBuiltin;
pub use crate::services_metrics::instant::ServicesMetricsInstant;
pub use crate::services_metrics::message::ServiceCallStats;

use crate::services_metrics::message::{ServiceMemoryStat, ServiceMetricsMsg};

#[derive(Clone)]
pub struct ServicesMetrics {
    pub instant: Option<ServicesMetricsInstant>,
    pub builtin: ServicesMetricsBuiltin,
    metrics_backend_outlet: Outlet<ServiceMetricsMsg>,
}

impl fmt::Debug for ServicesMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServicesMetrics").finish()
    }
}

impl ServicesMetrics {
    pub fn new(
        instant: Option<ServicesMetricsInstant>,
        metrics_backend_outlet: Outlet<ServiceMetricsMsg>,
    ) -> Self {
        Self {
            instant,
            builtin: ServicesMetricsBuiltin::new(),
            metrics_backend_outlet,
        }
    }

    pub fn with_instant_backend(
        timer_resolution: Duration,
        registry: &mut Registry,
    ) -> (ServicesMetricsBackend, Self) {
        let (outlet, inlet) = unbounded();

        let instant = ServicesMetricsInstant::new(registry);
        let memory_metrics = instant.memory_metrics.clone();

        let metrics = Self::new(Some(instant), outlet);
        let backend = ServicesMetricsBackend::with_instant_metrics(
            timer_resolution,
            memory_metrics,
            metrics.builtin.clone(),
            inlet,
        );
        (backend, metrics)
    }

    pub fn with_simple_backend() -> (ServicesMetricsBackend, Self) {
        let (outlet, inlet) = unbounded();
        let metrics = Self::new(None, outlet);
        let backend = ServicesMetricsBackend::new(metrics.builtin.clone(), inlet);
        (backend, metrics)
    }

    pub fn observe_service_state(
        &self,
        service_id: String,
        function_name: String,
        memory: MemoryStats,
        stats: ServiceCallStats,
    ) {
        self.observe_service_call(service_id.clone(), function_name, stats);
        if self.instant.is_some() {
            self.observe_service_mem(service_id, memory);
        }
    }

    pub fn observe_service_call(
        &self,
        service_id: String,
        function_name: String,
        stats: ServiceCallStats,
    ) {
        self.send(ServiceMetricsMsg::CallStats {
            service_id,
            function_name,
            stats,
        });
    }

    pub fn observe_service_call_unknown(&self, service_id: String, stats: ServiceCallStats) {
        let function_name = "<unknown>".to_string();
        self.send(ServiceMetricsMsg::CallStats {
            service_id,
            function_name,
            stats,
        });
    }

    /// Collect all metrics that are relevant on service creation.
    pub fn observe_created(&self, service_id: String, stats: MemoryStats) {
        if let Some(instant) = self.instant.as_ref() {
            instant.services_count.inc();
            instant
                .modules_in_services_count
                .observe(stats.0.len() as f64);
            self.observe_service_mem(service_id, stats);
        }
    }

    pub fn observe_instant<F>(&self, callback: F)
    where
        F: Fn(&ServicesMetricsInstant) -> (),
    {
        if let Some(instant) = self.instant.as_ref() {
            callback(instant);
        }
    }

    fn observe_service_mem(&self, service_id: String, stats: MemoryStats) {
        let msg = ServiceMetricsMsg::Memory {
            service_id,
            memory_stat: ServiceMemoryStat::new(stats),
        };
        self.send(msg);
    }

    fn send(&self, msg: ServiceMetricsMsg) {
        let result = self.metrics_backend_outlet.unbounded_send(msg);
        if let Err(e) = result {
            log::warn!("Can't save services' metrics: {:?}", e);
        }
    }
}
