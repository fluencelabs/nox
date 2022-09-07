mod connection_pool;
mod connectivity;
mod dispatcher;
mod network_protocol;
mod particle_executor;
mod services_metrics;
mod vm_pool;

pub use connection_pool::ConnectionPoolMetrics;
pub use connectivity::ConnectivityMetrics;
pub use connectivity::Resolution;
pub use dispatcher::DispatcherMetrics;
pub use particle_executor::{FunctionKind, ParticleExecutorMetrics};
use prometheus_client::encoding::text::SendSyncEncodeMetric;
use prometheus_client::registry::Registry;
pub use services_metrics::{
    ServiceCallStats, ServicesMetrics, ServicesMetricsBackend, ServicesMetricsBuiltin,
    ServicesMetricsExternal,
};
pub use vm_pool::VmPoolMetrics;

// TODO:
// - service heap statistics
// - interpreter heap histograms / summary
// - individual actor mailbox size: max and histogram
// - count 'Error processing inbound ProtocolMessage: unexpected end of file'
// - number of scheduled script executions

/// from 100 microseconds to 120 seconds
pub(self) fn execution_time_buckets() -> std::vec::IntoIter<f64> {
    vec![
        0.0001, 0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 7.0, 15.0, 30.0, 60.0, 120.0,
    ]
    .into_iter()
}

/// 1mib, 5mib, 10mib, 25mib, 50mib, 100mib, 200mib, 500mib, 1gib
pub(self) fn mem_buckets() -> std::vec::IntoIter<f64> {
    to_mib(vec![1, 5, 10, 25, 50, 100, 200, 500, 1024].into_iter())
}

/// 1mib, 5mib, 10mib, 25mib, 50mib, 100mib, 200mib, 500mib, 1gib, 2gib, 3gib, 4gib
pub(self) fn mem_buckets_4gib() -> std::vec::IntoIter<f64> {
    to_mib(vec![1, 5, 10, 25, 50, 100, 200, 500, 1024, 2048, 3072, 4096].into_iter())
}

/// 1mib, 5mib, 10mib, 25mib, 50mib, 100mib, 200mib, 500mib, 1gib, 2gib, 3gib, 4gib, 8gib
pub(self) fn mem_buckets_8gib() -> std::vec::IntoIter<f64> {
    to_mib(
        vec![
            1, 5, 10, 25, 50, 100, 200, 500, 1024, 2048, 3072, 4096, 8192,
        ]
        .into_iter(),
    )
}

fn to_mib(values: std::vec::IntoIter<u64>) -> std::vec::IntoIter<f64> {
    values
        .map(|n| bytesize::mib(n) as f64)
        .collect::<Vec<_>>()
        .into_iter()
}

pub(self) fn register<M>(registry: &mut Registry, metric: M, name: &str, help: &str) -> M
where
    M: 'static + SendSyncEncodeMetric + Clone,
{
    registry.register(name, help, Box::new(metric.clone()));
    metric
}
