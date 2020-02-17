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

use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use crate::node_service::p2p::events::P2PNetworkEvents;
use libp2p::floodsub::FloodsubEvent;
use libp2p::swarm::NetworkBehaviour;
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::PeerId;
use log::trace;
use parity_multiaddr::Multiaddr;
use std::str::FromStr;

impl NetworkBehaviourEventProcess<FloodsubEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                let p2p_message = serde_json::from_slice(&message.data).unwrap();
                match p2p_message {
                    P2PNetworkEvents::NodeConnected { node_id, peer_ids } => {
                        // here we are expecting that new node will connect to our by itself,
                        // because rust-libp2p supports now only one connection between listener
                        // and dialer - https://github.com/libp2p/rust-libp2p/issues/912.
                        // It is a poor design, but it seems that there are no other ways now.
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new node {} connected",
                            node_id
                        );

                        // converts Vec<Vec<u8>> to Vec<PeerId>
                        let peer_ids = peer_ids
                            .into_iter()
                            .map(|peer| PeerId::from_bytes(peer).unwrap())
                            .collect();
                        self.relay.add_new_node(node_id, peer_ids);
                        self.relay.print_network_state();
                    }

                    P2PNetworkEvents::NodeDisconnected { node_id } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!(
                            "node_service/p2p/behaviour/floodsub: new node {} connected",
                            node_id
                        );

                        self.relay.remove_node(&node_id);
                        self.relay.print_network_state()
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

                        self.relay.print_network_state();
                    }

                    P2PNetworkEvents::PeersDisconnected { node_id, peer_ids } => {
                        let node_id = PeerId::from_bytes(node_id).unwrap();
                        trace!("node_service/p2p/behaviour/floodsub: some peers disconnected from node {}", node_id);

                        for peer in peer_ids {
                            self.relay
                                .remove_peer(&node_id, &PeerId::from_bytes(peer).unwrap());
                        }

                        self.relay.print_network_state();
                    }

                    P2PNetworkEvents::NetworkState { network_map } => {
                        if self.initialized {
                            // pass the initialization step if we have already seen the NodesMap event
                            return;
                        }

                        self.initialized = true;

                        for (node_id, node_addrs, peers) in network_map {
                            let node_id = PeerId::from_bytes(node_id).unwrap();
                            if node_id == self.local_node_id {
                                // pass current node
                                continue;
                            }

                            if !node_addrs.is_empty() {
                                // converts Vec<String> to Vec<Multiaddr>
                                let node_addrs = node_addrs
                                    .iter()
                                    .map(|addr| Multiaddr::from_str(addr).unwrap())
                                    .collect();

                                trace!(
                                    "connect to node with peer id = {} and multiaddrs = {:?}",
                                    node_id,
                                    node_addrs
                                );

                                self.connect_to_node(node_id.clone(), node_addrs);
                            }

                            self.relay.add_new_node(
                                node_id,
                                peers
                                    .into_iter()
                                    .map(|peer_id| PeerId::from_bytes(peer_id).unwrap())
                                    .collect(),
                            )
                        }

                        self.relay.print_network_state();
                    }
                }
            }
            FloodsubEvent::Subscribed { .. } => {
                trace!("new node subscribed - send to it the whole network state");

                // new node is subscribed - send to it the whole network map
                let network_state = self.relay.network_state().clone();
                // convert from HashMap<PeerId, HashSet<PeerId>> to Vec<Vec<u8>, Vec<String>, Vec<Vec<u8>>>
                let mut network_map = network_state
                    .into_iter()
                    .map(|(node_id, peers)| {
                        (
                            node_id.clone().into_bytes(),
                            self.swarm_state
                                .addresses_of_peer(&node_id)
                                .iter()
                                .map(|addr| addr.to_string())
                                .collect(),
                            peers
                                .into_iter()
                                .map(|peer_id| peer_id.into_bytes())
                                .collect(),
                        )
                    })
                    .collect::<Vec<(Vec<u8>, Vec<String>, Vec<Vec<u8>>)>>();

                // add id and peers of the local node
                network_map.push((
                    self.local_node_id.clone().into_bytes(),
                    Vec::new(),
                    self.relay
                        .connected_peers()
                        .iter()
                        .cloned()
                        .map(|e| e.into_bytes())
                        .collect(),
                ));

                trace!(
                    "node_service/p2p/behaviour/swarm_state_event: gossip nodes map {:?}",
                    network_map
                );

                self.gossip_network_update(P2PNetworkEvents::NetworkState { network_map });
            }
            FloodsubEvent::Unsubscribed { .. } => {}
        }
    }
}
