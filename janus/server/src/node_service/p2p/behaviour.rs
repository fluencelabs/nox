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

use crate::node_service::p2p::events::P2PNetworkEvents;
use crate::node_service::p2p::swarm_state_behaviour::{SwarmStateBehaviour, SwarmStateEvent};
use crate::node_service::relay::{
    behaviour::{NetworkState, PeerRelayLayerBehaviour},
    events::RelayEvent,
};
use futures::task::Poll;
use futures::{AsyncRead, AsyncWrite};
use libp2p::core::either::EitherOutput;
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess};
use libp2p::{NetworkBehaviour, PeerId};
use log::trace;
use parity_multiaddr::Multiaddr;
use serde_json;
use std::collections::VecDeque;
use std::str::FromStr;

/// This type is constructed inside NetworkBehaviour proc macro and represents the InEvent type
/// parameter of NetworkBehaviourAction. Should be regenerated each time a set of behaviours
/// of the NodeServiceBehaviour is changed.
type NodeServiceBehaviourInEvent<Substream> = EitherOutput<EitherOutput<EitherOutput<EitherOutput
    <<<<libp2p::ping::Ping<Substream> as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent,
    <<<PeerRelayLayerBehaviour<Substream> as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<libp2p::identify::Identify<Substream> as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<libp2p::floodsub::Floodsub<Substream> as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<SwarmStateBehaviour<Substream> as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>;

/// Behaviour of the p2p layer that is responsible for keeping the network state actual and rules
/// all other protocols of the Janus.
#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", out_event = "RelayEvent")]
pub struct NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    ping: Ping<Substream>,
    relay: PeerRelayLayerBehaviour<Substream>,
    identity: Identify<Substream>,
    floodsub: Floodsub<Substream>,
    swarm_state: SwarmStateBehaviour<Substream>,

    #[behaviour(ignore)]
    churn_topic: Topic,

    #[behaviour(ignore)]
    local_node_id: PeerId,

    /// Contains events that need to be propagate to external caller.
    #[behaviour(ignore)]
    events: VecDeque<NetworkBehaviourAction<NodeServiceBehaviourInEvent<Substream>, RelayEvent>>,

    // true, if service've seen NodesMap event
    #[behaviour(ignore)]
    initialized: bool,
}

impl<Substream> NetworkBehaviourEventProcess<RelayEvent> for NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn inject_event(&mut self, event: RelayEvent) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));
    }
}

impl<Substream> NetworkBehaviourEventProcess<PingEvent> for NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream> NetworkBehaviourEventProcess<IdentifyEvent> for NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream> NetworkBehaviourEventProcess<FloodsubEvent> for NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                let p2p_message = serde_json::from_slice(&message.data).unwrap();
                match p2p_message {
                    P2PNetworkEvents::NodeConnected { node_id, peer_ids } => {
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
                    P2PNetworkEvents::NodeDisconnected { node_id } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new node {} connected",
                            node_id
                        );

                        self.relay.remove_node(&node_id);
                    }
                    P2PNetworkEvents::PeersConnected { node_id, peer_ids } => {
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
                    P2PNetworkEvents::PeersDisconnected { node_id, peer_ids } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!("node_service/p2p/behaviour/floodsub: some peers disconnected from node {}", node_id);

                        for peer in peer_ids {
                            self.relay
                                .remove_peer(&node_id, &PeerId::from_bytes(peer).unwrap());
                        }
                    }
                    P2PNetworkEvents::NetworkState {
                        node_addrs,
                        network_map,
                    } => {
                        if self.initialized {
                            // pass the intialization step if we have already seen the NodesMap event
                            return;
                        }
                        self.initialized = true;
                        trace!(
                            "node_service/p2p/behaviour/floodsub: received nodes map event {:?}",
                            node_addrs
                        );

                        let node_addrs = node_addrs
                            .iter()
                            .flat_map(|addrs| {
                                addrs.iter().map(|addr| Multiaddr::from_str(addr).unwrap())
                            })
                            .collect();

                        self.connect_to_nodes(node_addrs);

                        for (node_id, peers) in network_map {
                            self.relay.add_new_node(
                                PeerId::from_bytes(node_id).unwrap(),
                                peers
                                    .iter()
                                    .cloned()
                                    .map(|peer_id| PeerId::from_bytes(peer_id).unwrap())
                                    .collect(),
                            )
                        }

                        self.relay.print_network_state();
                    }
                }
            }
            FloodsubEvent::Subscribed { .. } => {}
            FloodsubEvent::Unsubscribed { .. } => {}
        }
    }
}

impl<Substream> NetworkBehaviourEventProcess<SwarmStateEvent> for NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
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

                let node_addrs: Vec<Vec<String>> = self
                    .relay
                    .connected_peers()
                    .iter()
                    .map(|peer| {
                        self.addresses_of_peer(peer)
                            .iter()
                            .map(|addr| addr.to_string())
                            .collect()
                    })
                    .collect();

                // convert from HashMap<PeerId, HashSet<PeerId>> to HashMap<Vec<u8>, HashSet<Vec<u8>>>
                let network_map = self
                    .relay
                    .network_state()
                    .iter()
                    .map(|(node_id, peers)| {
                        (
                            node_id.clone().into_bytes(),
                            peers
                                .iter()
                                .cloned()
                                .map(|peer_id| peer_id.into_bytes())
                                .collect(),
                        )
                    })
                    .collect::<Vec<(Vec<u8>, Vec<Vec<u8>>)>>();

                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: gossip nodes map {:?}",
                    node_addrs
                );

                self.gossip_network_update(P2PNetworkEvents::NetworkState {
                    node_addrs,
                    network_map,
                });
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

impl<Substream> NodeServiceBehaviour<Substream>
where
    Substream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(local_peer_id: PeerId, local_public_key: PublicKey, churn_topic: Topic) -> Self {
        let relay = PeerRelayLayerBehaviour::new();
        let ping = Ping::new(
            PingConfig::new()
                .with_max_failures(unsafe { core::num::NonZeroU32::new_unchecked(10) }),
        );
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
            events: VecDeque::new(),
            initialized: false,
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

    /*    pub fn exit(&mut self) {
            let message = P2PNetworkEvents::NodeDisconnected {
                node_id: self.local_node_id.clone().into_bytes(),
            };

            self.gossip_network_update(message);
        }
    */

    fn gossip_network_update(&mut self, message: P2PNetworkEvents) {
        let message =
            serde_json::to_vec(&message).expect("failed to convert gossip message to json");

        self.floodsub.publish(&self.churn_topic, message);
    }

    fn connect_to_nodes(&mut self, node_addrs: Vec<Multiaddr>) {
        for node_addr in node_addrs {
            self.events
                .push_back(NetworkBehaviourAction::DialAddress { address: node_addr })
        }
    }

    fn custom_poll(
        &mut self,
        _: &mut std::task::Context,
    ) -> Poll<NetworkBehaviourAction<NodeServiceBehaviourInEvent<Substream>, RelayEvent>> {
        if let Some(event) = self.events.pop_front() {
            // this events should be consumed during the node service polling
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
