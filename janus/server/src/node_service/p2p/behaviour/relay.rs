use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use crate::node_service::relay::events::RelayEvent;
use libp2p::swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess};

impl NetworkBehaviourEventProcess<RelayEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, event: RelayEvent) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));
    }
}
