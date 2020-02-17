use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use libp2p::identify::IdentifyEvent;
use libp2p::swarm::NetworkBehaviourEventProcess;

impl NetworkBehaviourEventProcess<IdentifyEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}
