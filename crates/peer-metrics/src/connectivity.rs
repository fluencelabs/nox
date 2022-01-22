use open_metrics_client::encoding::text::Encode;
use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::family::Family;
use open_metrics_client::registry::Registry;

#[derive(Encode, Hash, Clone, Eq, PartialEq)]
pub enum Resolution {
    Local,
    Kademlia,
    KademliaNotFound,
    KademliaError,
    ConnectionFailed,
}
#[derive(Encode, Hash, Clone, Eq, PartialEq)]
pub struct ResolutionLabel {
    action: Resolution,
}
#[derive(Clone)]
pub struct ConnectivityMetrics {
    contact_resolve: Family<ResolutionLabel, Counter>,
    pub particle_send_success: Counter,
    pub particle_send_failure: Counter,
    pub bootstrap_disconnected: Counter,
    pub bootstrap_connected: Counter,
}

impl ConnectivityMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("connectivity");

        let contact_resolve = Family::default();
        sub_registry.register(
            "contact_resolve",
            "Counters regarding contact resolution in particle processing",
            Box::new(contact_resolve.clone()),
        );

        let particle_send_success = Counter::default();
        sub_registry.register(
            "particle_send_success",
            "Number of sent particles",
            Box::new(particle_send_success.clone()),
        );

        let particle_send_failure = Counter::default();
        sub_registry.register(
            "particle_send_failure",
            "Number of errors on particle sending",
            Box::new(particle_send_failure.clone()),
        );

        let bootstrap_disconnected = Counter::default();
        sub_registry.register(
            "bootstrap_disconnected",
            "Number of times peer disconnected from bootstrap peers",
            Box::new(bootstrap_disconnected.clone()),
        );

        let bootstrap_connected = Counter::default();
        sub_registry.register(
            "bootstrap_connected",
            "Number of times peer connected (or reconnected) to a bootstrap peer",
            Box::new(bootstrap_connected.clone()),
        );

        Self {
            contact_resolve,
            particle_send_success,
            particle_send_failure,
            bootstrap_disconnected,
            bootstrap_connected,
        }
    }

    pub fn count_resolution(&self, resolution: Resolution) {
        self.contact_resolve
            .get_or_create(&ResolutionLabel { action: resolution })
            .inc();
    }
}
