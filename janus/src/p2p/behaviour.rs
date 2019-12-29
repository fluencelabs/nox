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

use crate::p2p::message::P2PNetworkMessage;
use crate::relay::behaviour::JanusRelayBehaviour;
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::{NetworkBehaviour, PeerId};
use serde_json;
use tokio::prelude::*;

#[derive(NetworkBehaviour)]
pub struct JanusBehaviour<Substream: AsyncRead + AsyncWrite> {
    mdns: Mdns<Substream>,
    ping: Ping<Substream>,
    relay: JanusRelayBehaviour<Substream>,
    identity: Identify<Substream>,
    floodsub: Floodsub<Substream>,

    #[behaviour(ignore)]
    churn_topic: Topic,

    #[behaviour(ignore)]
    local_peer_id: PeerId,
}

impl<Substream: AsyncWrite + AsyncRead> NetworkBehaviourEventProcess<MdnsEvent>
    for JanusBehaviour<Substream>
{
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _addr) in list {
                    // trace
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<()>
    for JanusBehaviour<Substream>
{
    fn inject_event(&mut self, _event: ()) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for JanusBehaviour<Substream>
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for JanusBehaviour<Substream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<FloodsubEvent>
    for JanusBehaviour<Substream>
{
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                let p2p_message = serde_json::from_slice(&message.data).unwrap();
                match p2p_message {
                    P2PNetworkMessage::PeerConnected { peer, nodes } => {
                        let nodes = nodes
                            .iter()
                            .cloned()
                            .map(|node| PeerId::from_bytes(node).unwrap())
                            .collect();
                        self.relay
                            .add_new_peer(PeerId::from_bytes(peer).unwrap(), nodes);
                    }
                    P2PNetworkMessage::PeerDisconnected { peer } => {
                        self.relay.remove_peer(PeerId::from_bytes(peer).unwrap());
                    }
                    P2PNetworkMessage::NodesConnected { peer, nodes } => {
                        let peer = PeerId::from_bytes(peer.clone()).unwrap();
                        for node in nodes {
                            self.relay
                                .add_new_node(&peer, PeerId::from_bytes(node).unwrap());
                        }
                    }
                    P2PNetworkMessage::NodesDisconnected { peer, nodes } => {
                        let peer = PeerId::from_bytes(peer.clone()).unwrap();
                        for node in nodes {
                            self.relay
                                .remove_node(&peer, PeerId::from_bytes(node).unwrap());
                        }
                    }
                }
            }
            FloodsubEvent::Subscribed { .. } => {}
            FloodsubEvent::Unsubscribed { .. } => {}
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> JanusBehaviour<Substream> {
    pub fn new(local_peer_id: PeerId, local_public_key: PublicKey, churn_topic: Topic) -> Self {
        let mdns = Mdns::new().expect("failed to create mdns");
        let relay = JanusRelayBehaviour::new();
        let ping = Ping::new(PingConfig::new());
        let floodsub = Floodsub::new(local_peer_id.clone());
        let identity = Identify::new("/janus/1.0.0".into(), "janus".into(), local_public_key);

        JanusBehaviour {
            mdns,
            ping,
            relay,
            identity,
            floodsub,
            churn_topic,
            local_peer_id,
        }
    }

    /// Sends peer_id and connected nodes to other network participants
    ///
    /// Currently uses floodsub protocol.
    pub fn gossip_peer_state(&mut self) {
        let nodes = self
            .relay
            .connected_nodes()
            .iter()
            .map(|peer| peer.as_bytes().to_vec())
            .collect();

        let message = P2PNetworkMessage::PeerConnected {
            peer: self.local_peer_id.clone().into_bytes(),
            nodes,
        };

        self.gossip_network_update(message);
    }

    pub fn add_connected_node(&mut self, node: PeerId) {
        self.relay.connected_nodes_mut().insert(node.clone());

        let message = P2PNetworkMessage::NodesConnected {
            peer: self.local_peer_id.clone().into_bytes(),
            nodes: vec![node.into_bytes()],
        };

        self.gossip_network_update(message);
    }

    pub fn remove_connected_node(&mut self, node: PeerId) {
        self.relay.connected_nodes_mut().remove(&node);

        let message = P2PNetworkMessage::NodesDisconnected {
            peer: self.local_peer_id.clone().into_bytes(),
            nodes: vec![node.into_bytes()],
        };

        self.gossip_network_update(message);
    }

    pub fn exit(&mut self) {
        let message = P2PNetworkMessage::PeerDisconnected {
            peer: self.local_peer_id.clone().into_bytes(),
        };

        self.gossip_network_update(message);
    }

    fn gossip_network_update(&mut self, message: P2PNetworkMessage) {
        let message =
            serde_json::to_vec(&message).expect("failed to convert gossip message to json");

        self.floodsub.publish(&self.churn_topic, message);
    }
}
