/*
 * Copyright 2019 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

mod floodsub;
mod identity;
mod ping;
mod relay;
mod swarm_state;

use crate::event_polling;
use crate::node_service::p2p::events::P2PNetworkEvents;
use crate::node_service::p2p::swarm_state_behaviour::SwarmStateBehaviour;
use crate::node_service::relay::{
    behaviour::{NetworkState, PeerRelayLayerBehaviour},
    events::RelayEvent,
};
use libp2p::core::either::EitherOutput;
use libp2p::floodsub::{Floodsub, Topic};
use libp2p::identify::Identify;
use libp2p::identity::PublicKey;
use libp2p::ping::{Ping, PingConfig};
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::{NetworkBehaviour, PeerId};
use log::trace;
use parity_multiaddr::Multiaddr;
use serde_json;
use std::collections::{HashSet, VecDeque};

/// This type is constructed inside NetworkBehaviour proc macro and represents the InEvent type
/// parameter of NetworkBehaviourAction. Should be regenerated each time a set of behaviours
/// of the NodeServiceBehaviour is changed.
type NodeServiceBehaviourInEvent = EitherOutput<EitherOutput<EitherOutput<EitherOutput
    <<<<libp2p::ping::Ping as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent,
    <<<PeerRelayLayerBehaviour as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<libp2p::identify::Identify as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<libp2p::floodsub::Floodsub as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<SwarmStateBehaviour as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>;

/// Behaviour of the p2p layer that is responsible for keeping the network state actual and rules
/// all other protocols of the Janus.
#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", out_event = "RelayEvent")]
pub struct NodeServiceBehaviour {
    ping: Ping,
    relay: PeerRelayLayerBehaviour,
    identity: Identify,
    floodsub: Floodsub,
    swarm_state: SwarmStateBehaviour,

    #[behaviour(ignore)]
    churn_topic: Topic,

    #[behaviour(ignore)]
    local_node_id: PeerId,

    /// Contains events that need to be propagate to external caller.
    #[behaviour(ignore)]
    events: VecDeque<NetworkBehaviourAction<NodeServiceBehaviourInEvent, RelayEvent>>,

    // true, if a service's seen NodesMap event
    #[behaviour(ignore)]
    initialized: bool,
}

impl NodeServiceBehaviour {
    pub fn new(
        local_peer_id: PeerId,
        local_public_key: PublicKey,
        churn_topic: Topic,
        node_service_port: u16,
    ) -> Self {
        let ping = Ping::new(
            PingConfig::new()
                .with_max_failures(unsafe { std::num::NonZeroU32::new_unchecked(10) })
                .with_keep_alive(true),
        );
        let relay = PeerRelayLayerBehaviour::new();
        let mut floodsub = Floodsub::new(local_peer_id.clone());
        let identity = Identify::new("/janus/p2p/1.0.0".into(), "0.1.0".into(), local_public_key);
        let swarm_state = SwarmStateBehaviour::new(node_service_port);

        floodsub.subscribe(churn_topic.clone());

        Self {
            ping,
            relay,
            identity,
            floodsub,
            swarm_state,
            churn_topic,
            local_node_id: local_peer_id,
            events: VecDeque::new(),
            initialized: false,
        }
    }

    /// Sends peer_id and connected nodes to other network participants
    ///
    /// Currently uses floodsub protocol.
    pub fn gossip_node_state(&mut self) {
        let peer_ids = self
            .relay
            .connected_peers()
            .iter()
            .map(|peer| peer.as_bytes().to_vec())
            .collect();

        let message = P2PNetworkEvents::NodeConnected {
            node_id: self.local_node_id.clone().into_bytes(),
            peer_ids,
        };

        self.gossip_network_update(message);
    }

    pub fn add_connected_peer(&mut self, peer_id: PeerId) {
        trace!(
            "node_service/p2p/behaviour: add connected peer {:?}",
            peer_id
        );

        self.relay.add_local_peer(peer_id.clone());
        self.relay.print_network_state();

        let message = P2PNetworkEvents::PeersConnected {
            node_id: self.local_node_id.clone().into_bytes(),
            peer_ids: vec![peer_id.into_bytes()],
        };

        self.gossip_network_update(message);
    }

    pub fn remove_connected_peer(&mut self, peer_id: PeerId) {
        trace!(
            "node_service/p2p/behaviour: remove connected peer {:?}",
            peer_id
        );

        self.relay.remove_local_peer(&peer_id);
        self.relay.print_network_state();

        let message = P2PNetworkEvents::PeersDisconnected {
            node_id: self.local_node_id.clone().into_bytes(),
            peer_ids: vec![peer_id.into_bytes()],
        };

        self.gossip_network_update(message);
    }

    pub fn relay(&mut self, relay_message: RelayEvent) {
        self.relay.relay(relay_message);
    }

    pub fn network_state(&self) -> &NetworkState {
        self.relay.network_state()
    }

    #[allow(dead_code)]
    pub fn exit(&mut self) {
        unimplemented!("need to decide how exactly NodeDisconnect message will be send");
    }

    fn gossip_network_update(&mut self, message: P2PNetworkEvents) {
        let message =
            serde_json::to_vec(&message).expect("failed to convert gossip message to json");

        self.floodsub.publish(self.churn_topic.clone(), message);
    }

    fn connect_to_node(&mut self, node_id: PeerId, node_addrs: Vec<Multiaddr>) {
        use std::iter::FromIterator;

        let addrs = HashSet::from_iter(node_addrs);
        self.swarm_state.add_node_addresses(node_id.clone(), addrs);

        self.events
            .push_back(NetworkBehaviourAction::DialPeer { peer_id: node_id })
    }

    // produces RelayEvent
    event_polling!(
        custom_poll,
        events,
        NetworkBehaviourAction<NodeServiceBehaviourInEvent, RelayEvent>
    );
}
