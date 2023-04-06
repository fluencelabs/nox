use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

pub fn add_info_metrics(
    registry: &mut Registry,
    node_version: String,
    air_version: String,
    spell_version: String,
) {
    let sub_registry = registry.sub_registry_with_prefix("rust_peer");
    let info = Info::new(vec![
        ("peer-version", node_version),
        ("air-version", air_version),
        ("spell-version", spell_version),
    ]);
    sub_registry.register("version", "Rust Peer versions", info);
}
