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

use crate::config::NodeServiceConfig;
use crate::node_service::{
    p2p::{
        behaviour::NodeServiceBehaviour, transport::build_transport,
        transport::NodeServiceTransport,
    },
    relay::events::RelayEvent,
};
use crate::peer_service::libp2p::notifications::{InPeerNotification, OutPeerNotification};
use async_std::task;
use futures::channel::{mpsc, oneshot};
use futures::{select, stream::StreamExt};
use futures_util::future::FutureExt;
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::{error, trace};
use parity_multiaddr::{Multiaddr, Protocol};

pub struct NodeService {
    pub swarm: Box<
        Swarm<
            NodeServiceTransport,
            NodeServiceBehaviour<SubstreamRef<std::sync::Arc<StreamMuxerBox>>>,
        >,
    >,
}

impl NodeService {
    pub fn new(config: NodeServiceConfig) -> Self {
        let local_key = match config.key_pair {
            Some(kp) => kp,
            None => identity::Keypair::generate_ed25519(),
        };
        let local_peer_id = PeerId::from(local_key.public());
        println!("node service is starting with id = {}", local_peer_id);

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let churn_topic = config.churn_topic;
            let behaviour = NodeServiceBehaviour::new(
                local_peer_id.clone(),
                local_key.public(),
                churn_topic,
                config.listen_port,
            );

            Box::new(Swarm::new(transport, behaviour, local_peer_id.clone()))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        for addr in config.bootstrap_nodes {
            Swarm::dial_addr(&mut swarm, addr).expect("dialed to bootstrap node failed");
        }

        swarm.gossip_node_state();

        Self { swarm }
    }
}

fn handle_peer_notification(
    notification: OutPeerNotification,
    node_service: &mut NodeService,
    peer_service_in_sender: &mpsc::UnboundedSender<InPeerNotification>,
) {
    match notification {
        OutPeerNotification::PeerConnected { peer_id } => {
            node_service.swarm.add_connected_peer(peer_id)
        }

        OutPeerNotification::PeerDisconnected { peer_id } => {
            node_service.swarm.remove_connected_peer(peer_id)
        }

        OutPeerNotification::Relay {
            src_id,
            dst_id,
            data,
        } => node_service.swarm.relay(RelayEvent {
            src_id: src_id.into_bytes(),
            dst_id: dst_id.into_bytes(),
            data,
        }),

        OutPeerNotification::GetNetworkState { src_id } => {
            let network_state = node_service.swarm.network_state();
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
    }
}

pub fn start_node_service(
    mut node_service: NodeService,
    mut peer_service_out_receiver: mpsc::UnboundedReceiver<OutPeerNotification>,
    peer_service_in_sender: mpsc::UnboundedSender<InPeerNotification>,
) -> oneshot::Sender<()> {
    let (exit_sender, exit_receiver) = oneshot::channel();
    let mut exit_receiver = exit_receiver.into_stream();

    task::spawn(async move {
        loop {
            select! {
                from_peer = peer_service_out_receiver.next().fuse() => {
                    match from_peer {
                        Some(notification) => handle_peer_notification(
                            notification,
                            &mut node_service,
                            &peer_service_in_sender,
                        ),

                        // channel is closed when peer service was shut down - does nothing
                        // (node service is main service and could run without peer service)
                        None => {},
                    }
                },

                from_swarm = node_service.swarm.next().fuse() => {
                    match from_swarm {
                        Some(event) => {
                            trace!("node_service/select: sending {:?} to peer_service", event);

                            peer_service_in_sender
                                .unbounded_send(InPeerNotification::Relay {
                                    src_id: PeerId::from_bytes(event.src_id).unwrap(),
                                    dst_id: PeerId::from_bytes(event.dst_id).unwrap(),
                                    data: event.data,
                                })
                                .unwrap();
                        },
                        None => {
                            error!("node_service/select: swarm stream has unexpectedly ended");

                            // swarm is closed - break the loop
                            break;
                        }
                    }
                },

                _ = exit_receiver.next().fuse() => {
                    break
                }
            }
        }
    });

    exit_sender
}
