use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use crate::node_service::p2p::swarm_state_behaviour::SwarmStateEvent;
use libp2p::swarm::NetworkBehaviourEventProcess;
use log::trace;

impl NetworkBehaviourEventProcess<SwarmStateEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, event: SwarmStateEvent) {
        match event {
            SwarmStateEvent::Connected { id } => {
                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: new node {} connected",
                    id
                );
                self.floodsub.add_node_to_partial_view(id.clone());
                self.relay.add_new_node(id, Vec::new());
                self.relay.print_network_state();
            }
            SwarmStateEvent::Disconnected { id } => {
                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: node {} disconnected",
                    id
                );
                self.floodsub.remove_node_from_partial_view(&id);
                self.relay.remove_node(&id);
                self.relay.print_network_state();
            }
        }
    }
}
