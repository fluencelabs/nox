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

use crate::node_service::p2p::message::P2PNetworkMessage;
use crate::node_service::p2p::swarm_state_behaviour::{SwarmStateBehaviour, SwarmStateEvent};
use crate::node_service::relay::{
    behaviour::{NetworkState, PeerRelayLayerBehaviour},
    message::RelayMessage,
};
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::{NetworkBehaviour, PeerId};
use log::trace;
use serde_json;
use std::collections::VecDeque;
use tokio::prelude::*;

/// Behaviour of the p2p layer that is responsible for keeping the network state actual and rules
/// all other protocols of the Janus.
#[derive(NetworkBehaviour)]
pub struct NodeServiceBehaviour<Substream: AsyncRead + AsyncWrite> {
    ping: Ping<Substream>,
    relay: PeerRelayLayerBehaviour<Substream>,
    identity: Identify<Substream>,
    floodsub: Floodsub<Substream>,
    swarm_state: SwarmStateBehaviour<Substream>,

    #[behaviour(ignore)]
    churn_topic: Topic,

    #[behaviour(ignore)]
    local_node_id: PeerId,

    /// Relay messages that need to served to specified nodes.
    #[behaviour(ignore)]
    nodes_messages: VecDeque<RelayMessage>,
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<RelayMessage>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: RelayMessage) {
        self.nodes_messages.push_back(event);
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<FloodsubEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                let p2p_message = serde_json::from_slice(&message.data).unwrap();
                match p2p_message {
                    P2PNetworkMessage::NodeConnected { node_id, peer_ids } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new node {} connected",
                            node_id
                        );

                        // converts Vec<Vec<u8>> to Vec<PeerId>
                        let peer_ids = peer_ids
                            .iter()
                            .cloned()
                            .map(|peer| PeerId::from_bytes(peer).unwrap())
                            .collect();
                        self.relay.add_new_node(node_id, peer_ids);
                    }
                    P2PNetworkMessage::NodeDisconnected { node_id } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new node {} connected",
                            node_id
                        );

                        self.relay.remove_node(&node_id);
                    }
                    P2PNetworkMessage::PeersConnected { node_id, peer_ids } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new peers connected to node {}",
                            node_id
                        );

                        for peer_id in peer_ids {
                            self.relay
                                .add_new_peer(&node_id, PeerId::from_bytes(peer_id).unwrap());
                        }
                    }
                    P2PNetworkMessage::PeersDisconnected { node_id, peer_ids } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!("node_service/p2p/behaviour/floodsub: some peers disconnected from node {}", node_id);

                        for peer in peer_ids {
                            self.relay
                                .remove_peer(&node_id, &PeerId::from_bytes(peer).unwrap());
                        }
                    }
                }
            }
            FloodsubEvent::Subscribed { .. } => {}
            FloodsubEvent::Unsubscribed { .. } => {}
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<SwarmStateEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: SwarmStateEvent) {
        match event {
            SwarmStateEvent::Connected { id } => {
                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: new node {} connected",
                    id
                );
                self.floodsub.add_node_to_partial_view(id.clone());
                self.relay.add_new_node(id, Vec::new());
            }
            SwarmStateEvent::Disconnected { id } => {
                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: node {} disconnected",
                    id
                );
                self.floodsub.remove_node_from_partial_view(&id);
                self.relay.remove_node(&id);
            }
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NodeServiceBehaviour<Substream> {
    pub fn new(local_peer_id: PeerId, local_public_key: PublicKey, churn_topic: Topic) -> Self {
        let relay = PeerRelayLayerBehaviour::new();
        let ping = Ping::new(PingConfig::new());
        let mut floodsub = Floodsub::new(local_peer_id.clone());
        let identity = Identify::new("/janus/p2p/1.0.0".into(), "0.1.0".into(), local_public_key);
        let swarm_state = SwarmStateBehaviour::new();

        floodsub.subscribe(churn_topic.clone());

        Self {
            ping,
            relay,
            identity,
            floodsub,
            swarm_state,
            churn_topic,
            local_node_id: local_peer_id,
            nodes_messages: VecDeque::new(),
        }
    }

    /// Sends peer_id and connected nodes to other network participants
    ///
    /// Currently uses floodsub protocol.
    pub fn gossip_peer_state(&mut self) {
        let peer_ids = self
            .relay
            .connected_peers()
            .iter()
            .map(|peer| peer.as_bytes().to_vec())
            .collect();

        let message = P2PNetworkMessage::NodeConnected {
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

        self.relay.connected_peers_mut().insert(peer_id.clone());

        let message = P2PNetworkMessage::PeersConnected {
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

        self.relay.connected_peers_mut().remove(&peer_id);

        let message = P2PNetworkMessage::PeersDisconnected {
            node_id: self.local_node_id.clone().into_bytes(),
            peer_ids: vec![peer_id.into_bytes()],
        };

        self.gossip_network_update(message);
    }

    pub fn pop_peer_relay_message(&mut self) -> Option<RelayMessage> {
        self.nodes_messages.pop_front()
    }

    pub fn relay(&mut self, relay_message: RelayMessage) {
        self.relay.relay(relay_message);
    }

    pub fn network_state(&self) -> &NetworkState {
        self.relay.network_state()
    }

    pub fn exit(&mut self) {
        let message = P2PNetworkMessage::NodeDisconnected {
            node_id: self.local_node_id.clone().into_bytes(),
        };

        self.gossip_network_update(message);
    }

    fn gossip_network_update(&mut self, message: P2PNetworkMessage) {
        let message =
            serde_json::to_vec(&message).expect("failed to convert gossip message to json");

        self.floodsub.publish(&self.churn_topic, message);
    }
}
