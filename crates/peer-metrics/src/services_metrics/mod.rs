pub mod backend;
pub mod builtin;
pub mod external;
pub mod message;

use std::{fmt, time::Duration};

use prometheus_client::registry::Registry;

use futures::channel::mpsc::unbounded;

use fluence_app_service::ModuleDescriptor;
use fluence_libp2p::types::Outlet;

pub use crate::services_metrics::backend::ServicesMetricsBackend;
pub use crate::services_metrics::builtin::ServicesMetricsBuiltin;
pub use crate::services_metrics::external::ServicesMetricsExternal;
pub use crate::services_metrics::message::{ServiceCallStats, ServiceMemoryStat};

use crate::services_metrics::message::ServiceMetricsMsg;

#[derive(Clone)]
pub struct ServicesMetrics {
    pub external: Option<ServicesMetricsExternal>,
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
        external: Option<ServicesMetricsExternal>,
        metrics_backend_outlet: Outlet<ServiceMetricsMsg>,
        max_builtin_storage_size: usize,
    ) -> Self {
        Self {
            external,
            builtin: ServicesMetricsBuiltin::new(max_builtin_storage_size),
            metrics_backend_outlet,
        }
    }

    pub fn with_external_backend(
        timer_resolution: Duration,
        max_builtin_storage_size: usize,
        registry: &mut Registry,
    ) -> (ServicesMetricsBackend, Self) {
        let (outlet, inlet) = unbounded();

        let external = ServicesMetricsExternal::new(registry);
        let memory_metrics = external.memory_metrics.clone();

        let metrics = Self::new(Some(external), outlet, max_builtin_storage_size);
        let backend = ServicesMetricsBackend::with_external_metrics(
            timer_resolution,
            memory_metrics,
            metrics.builtin.clone(),
            inlet,
        );
        (backend, metrics)
    }

    pub fn with_simple_backend(max_builtin_storage_size: usize) -> (ServicesMetricsBackend, Self) {
        let (outlet, inlet) = unbounded();
        let metrics = Self::new(None, outlet, max_builtin_storage_size);
        let backend = ServicesMetricsBackend::new(metrics.builtin.clone(), inlet);
        (backend, metrics)
    }

    pub fn observe_service_state(
        &self,
        service_id: String,
        function_name: String,
        memory: ServiceMemoryStat,
        stats: ServiceCallStats,
    ) {
        self.observe_service_call(service_id.clone(), Some(function_name), stats);
        if self.external.is_some() {
            self.observe_service_mem(service_id, memory);
        }
    }

    pub fn observe_service_call(
        &self,
        service_id: String,
        function_name: Option<String>,
        stats: ServiceCallStats,
    ) {
        let function_name = function_name.unwrap_or("<unknown>".to_string());
        self.send(ServiceMetricsMsg::CallStats {
            service_id,
            function_name,
            stats,
        });
    }

    /// Collect all metrics that are relevant on service creation.
    pub fn observe_created(
        &self,
        service_id: String,
        stats: ServiceMemoryStat,
        creation_time: f64,
    ) {
        self.observe_external(move |external| {
            external.observe_created(stats.modules_stats.len() as f64, creation_time);
            self.observe_service_mem(service_id, stats);
        });
    }

    pub fn observe_removed(&self, removal_time: f64) {
        self.observe_external(|external| {
            external.observe_removed(removal_time);
        });
    }

    pub fn observe_service_config(&self, max_heap_size: u64, modules_config: &[ModuleDescriptor]) {
        self.observe_external(|external| {
            external.observe_service_max_mem(max_heap_size, modules_config);
        });
    }

    pub fn observe_external<F>(&self, callback: F)
    where
        F: FnOnce(&ServicesMetricsExternal),
    {
        if let Some(external) = self.external.as_ref() {
            callback(external);
        }
    }

    fn observe_service_mem(&self, service_id: String, stats: ServiceMemoryStat) {
        let msg = ServiceMetricsMsg::Memory {
            service_id,
            memory_stat: stats,
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
