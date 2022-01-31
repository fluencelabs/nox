use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct DispatcherMetrics {
    pub expired_particles: Counter,
}

impl DispatcherMetrics {
    pub fn new(registry: &mut Registry, _parallelism: Option<usize>) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("dispatcher");

        // TODO: prometheus doesn't parse this Info metric. Find a way to make it work.
        //       Gauge would work, but maybe it's possible to make Info work as well?
        // // NOTE: it MUST by a Vec of (String, String) or it would generate gibberish!
        // let parallelism: Info<Vec<(String, String)>> = Info::new(vec![(
        //     "particle_parallelism".to_string(),
        //     parallelism.map_or("unlimited".to_string(), |p| p.to_string()),
        // )]);
        // sub_registry.register(
        //     "particle_parallelism",
        //     "limit of simultaneously processed particles",
        //     Box::new(parallelism),
        // );

        let expired_particles = Counter::default();
        sub_registry.register(
            "particles_expired",
            "Number of particles expired by TTL",
            Box::new(expired_particles.clone()),
        );

        DispatcherMetrics { expired_particles }
    }
}
