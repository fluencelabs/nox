use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

pub fn add_info_metrics(
    registry: &mut Registry,
    node_version: String,
    air_version: String,
    spell_version: String,
) {
    let sub_registry = registry.sub_registry_with_prefix("nox");
    let info = Info::new(vec![
        ("node_version", node_version),
        ("air_version", air_version),
        ("spell_version", spell_version),
    ]);
    sub_registry.register("build", "Nox Info", info);
}
