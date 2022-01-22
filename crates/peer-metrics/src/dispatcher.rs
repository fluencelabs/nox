use open_metrics_client::encoding::text::encode;
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

#[test]
fn encode_info() {
    let mut registry = Registry::default();
    // let info = Info::new(vec![("os".to_string(), "GNU/linux".to_string())]);
    let info = Info::new(vec![(
        "particle_parallelism".to_string(),
        Some(10).map_or("unlimited".to_string(), |p| p.to_string()),
    )]);
    registry.register("my_info_metric", "My info metric", info);

    let mut encoded = Vec::new();
    encode(&mut encoded, &registry).unwrap();

    let expected = "# HELP my_info_metric My info metric.\n".to_owned()
        + "# TYPE my_info_metric info\n"
        + "my_info_metric_info{os=\"GNU/linux\"} 1\n"
        + "# EOF\n";

    let result = String::from_utf8(encoded.clone()).unwrap();
    // assert_eq!(expected, result);
    println!("{}", result);
}
