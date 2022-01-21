use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::info::Info;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct DispatcherMetrics {
    pub expired_particles: Counter,
}

impl DispatcherMetrics {
    pub fn new(registry: &mut Registry, parallelism: Option<usize>) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("dispatcher");

        let parallelism = Info::new(vec![
            "particle_parallelism".to_string(),
            parallelism.map_or("unlimited".to_string(), |p| p.to_string()),
        ]);
        sub_registry.register(
            "particle_parallelism",
            "limit of simultaneously processed particles",
            Box::new(parallelism),
        );

        let expired_particles = Counter::default();
        sub_registry.register(
            "particles_expired",
            "Number of particles expired by TTL",
            Box::new(expired_particles.clone()),
        );

        DispatcherMetrics { expired_particles }
    }
}
