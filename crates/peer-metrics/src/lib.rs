mod connection_pool;
mod connectivity;
mod dispatcher;
mod network_protocol;
mod particle_executor;
mod vm_pool;

pub use connection_pool::ConnectionPoolMetrics;
pub use connectivity::ConnectivityMetrics;
pub use connectivity::Resolution;
pub use dispatcher::DispatcherMetrics;
pub use particle_executor::{ParticleExecutorMetrics, ServiceCall};
pub use vm_pool::VmPoolMetrics;

// TODO:
// - service creation time
// - number of services
// - heap statistics
// - individual actor mailbox size: max and histogram
// - count 'Error processing inbound ProtocolMessage: unexpected end of file'
