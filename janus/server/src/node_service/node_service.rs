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

use crate::config::config::NodeServiceConfig;
use crate::node_service::{
    p2p::{
        behaviour::NodeServiceBehaviour, transport::build_transport,
    },
    relay::events::RelayEvent,
};
use crate::peer_service::libp2p::notifications::{InPeerNotification, OutPeerNotification};
use async_std::task;
use futures::channel::{mpsc, oneshot};
use futures::{select, stream::StreamExt};
use futures_util::future::FutureExt;
use libp2p::{
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};

type NodeServiceSwarm = Swarm<NodeServiceBehaviour>;

pub struct NodeService {
    pub swarm: Box<NodeServiceSwarm>,
    config: NodeServiceConfig,
}

impl NodeService {
    pub fn new(config: NodeServiceConfig) -> Self {
        let NodeServiceConfig {
            socket_timeout,
            churn_topic,
            key_pair,
            ..
        } = config.clone();

        let local_key = match key_pair {
            Some(kp) => kp,
            None => identity::Keypair::generate_ed25519(),
        };
        let local_peer_id = PeerId::from(local_key.public());
        println!("node service is starting with id = {}", local_peer_id);

        let swarm = {
            let transport = build_transport(local_key.clone(), socket_timeout);
            let behaviour = NodeServiceBehaviour::new(
                local_peer_id.clone(),
                local_key.public(),
                churn_topic,
                config.listen_port,
            );

            Box::new(Swarm::new(transport, behaviour, local_peer_id))
        };

        Self { swarm, config }
    }

    pub fn start(
        mut self,
        peer_service_out_receiver: mpsc::UnboundedReceiver<OutPeerNotification>,
        peer_service_in_sender: mpsc::UnboundedSender<InPeerNotification>,
    ) -> oneshot::Sender<()> {
        let (exit_sender, exit_receiver) = oneshot::channel();

        self.prepare_node();

        task::spawn(async move {
            // fusing streams
            let mut peer_service_out_receiver = peer_service_out_receiver.fuse();
            let mut node_service_swarm = self.swarm.fuse();
            let mut exit_receiver = exit_receiver.into_stream().fuse();

            loop {
                select! {
                    from_peer = peer_service_out_receiver.next() => {
                        NodeService::handle_peer_notification(
                            node_service_swarm.get_mut(),
                            from_peer,
                            &peer_service_in_sender,
                        )
                    },

                    // swarm stream never ends
                    from_swarm = node_service_swarm.select_next_some() => {
                        trace!("node_service/select: sending {:?} to peer_service", from_swarm);

                        peer_service_in_sender
                            .unbounded_send(InPeerNotification::Relay {
                                src_id: PeerId::from_bytes(from_swarm.src_id).unwrap(),
                                dst_id: PeerId::from_bytes(from_swarm.dst_id).unwrap(),
                                data: from_swarm.data,
                            })
                            .unwrap();
                    },

                    _ = exit_receiver.next() => {
                        break
                    }
                }
            }
        });

        exit_sender
    }

    /// Prepares node before running.
    ///
    /// Preparing includes these steps:
    ///  - running swarm listener on address from the config
    ///  - dialing to bootstrap nodes
    ///  - gossiping node state
    #[inline]
    fn prepare_node(&mut self) {
        let mut listen_addr = Multiaddr::from(self.config.listen_ip);
        listen_addr.push(Protocol::Tcp(self.config.listen_port));

        Swarm::listen_on(&mut self.swarm, listen_addr).unwrap();

        for addr in &self.config.bootstrap_nodes {
            Swarm::dial_addr(&mut self.swarm, addr.clone())
                .expect("dialed to bootstrap node failed");
        }

        self.swarm.gossip_node_state();
    }

    /// Handles notifications from a peer service.
    #[inline]
    fn handle_peer_notification(
        swarm: &mut NodeServiceSwarm,
        notification: Option<OutPeerNotification>,
        peer_service_in_sender: &mpsc::UnboundedSender<InPeerNotification>,
    ) {
        match notification {
            Some(OutPeerNotification::PeerConnected { peer_id }) => {
                swarm.add_connected_peer(peer_id)
            }

            Some(OutPeerNotification::PeerDisconnected { peer_id }) => {
                swarm.remove_connected_peer(peer_id)
            }

            Some(OutPeerNotification::Relay {
                src_id,
                dst_id,
                data,
            }) => swarm.relay(RelayEvent {
                src_id: src_id.into_bytes(),
                dst_id: dst_id.into_bytes(),
                data,
            }),

            Some(OutPeerNotification::GetNetworkState { src_id }) => {
                let network_state = swarm.network_state();
                let network_state = network_state
                    .iter()
                    .map(|v| v.0.clone())
                    .collect::<Vec<PeerId>>();

                peer_service_in_sender
                    .unbounded_send(InPeerNotification::NetworkState {
                        dst_id: src_id,
                        state: network_state,
                    })
                    .expect("Failed during send event to the node service");
            }

            // channel is closed when peer service was shut down - does nothing
            // (node service is main service and could run without peer service)
            None => {
                trace!("trying to poll closed channel from the peer service");
            }
        }
    }
}
