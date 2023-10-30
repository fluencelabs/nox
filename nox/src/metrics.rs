use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use std::time::Duration;

#[derive(Clone)]
#[allow(dead_code)]
pub struct TokioMetrics {
    workers_count: Gauge,
    total_park_count: Counter,
}

pub struct TokioMetricsConfig {
    pub tick_duration: Duration,
}

impl TokioMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("tokio");

        let workers_count = Gauge::default();

        sub_registry.register(
            "workers_count",
            "The number of worker threads used by the runtime.",
            workers_count.clone(),
        );

        let total_park_count = Counter::default();

        sub_registry.register(
            "total_park_count",
            "The number of times worker threads parked.",
            workers_count.clone(),
        );

        Self {
            workers_count,
            total_park_count,
        }
    }

    pub fn start(&self, config: TokioMetricsConfig) {
        let handle = tokio::runtime::Handle::current();
        let runtime_monitor = tokio_metrics::RuntimeMonitor::new(&handle);

        //let workers_count = self.workers_count.clone();
       // let total_park_count = self.total_park_count.clone();

        tokio::task::spawn(async move {
            for metrics in runtime_monitor.intervals() {
                log::info!("Metrics {:?}", metrics);
                tokio::time::sleep(config.tick_duration).await;
            }
        });
    }
}
