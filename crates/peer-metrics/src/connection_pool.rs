use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::gauge::Gauge;
use open_metrics_client::metrics::histogram::Histogram;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct ConnectionPoolMetrics {
    pub received_particles: Counter,
    pub particle_sizes: Histogram,
    pub connected_peers: Gauge,
    pub particle_queue_size: Gauge,
}

impl ConnectionPoolMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("connection_pool");

        let received_particles = Counter.default();
        sub_registry.register(
            "received_particles",
            "Number of particles received from the network (not unique)",
            Box::new(received_particles.clone()),
        );

        let particle_sizes = Histogram.default();
        sub_registry.register(
            "particle_sizes",
            "Distribution of particle data sizes",
            Box::new(particle_sizes.clone()),
        );

        let connected_peers = Gauge.default();
        sub_registry.register(
            "connected_peers",
            "Number of peers we have connections to at a given moment",
            Box::new(connected_peers.clone()),
        );

        let particle_queue_size = Gauge.default();
        sub_registry.register(
            "particle_queue_size",
            "Size of a particle queue in connection pool",
            Box::new(particle_queue_size.clone()),
        );

        Self {
            received_particles,
            particle_sizes,
            connected_peers,
            particle_queue_size,
        }
    }
}
