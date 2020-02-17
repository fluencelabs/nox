use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use libp2p::ping::PingEvent;
use libp2p::swarm::NetworkBehaviourEventProcess;

impl NetworkBehaviourEventProcess<PingEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, event: PingEvent) {
        if event.result.is_err() {
            log::debug!("ping failed with {:?}", event);
        }
    }
}
