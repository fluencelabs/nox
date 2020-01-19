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
use crate::peer_service::notifications::{InPeerNotification, OutPeerNotification};
use async_std::task;
use futures::channel::{mpsc, oneshot};
use futures::stream::StreamExt;
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub struct NodeService {
    pub swarm:
        Box<Swarm<NodeServiceTransport, NodeServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl NodeService {
    pub fn new(config: NodeServiceConfig) -> Arc<Mutex<Self>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("node service is starting with id = {}", local_peer_id);

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let churn_topic = config.churn_topic;
            let behaviour =
                NodeServiceBehaviour::new(local_peer_id.clone(), local_key.public(), churn_topic);

            Box::new(Swarm::new(transport, behaviour, local_peer_id.clone()))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        for addr in config.bootstrap_nodes {
            Swarm::dial_addr(&mut swarm, addr).expect("dialed to bootstrap node failed");
        }

        swarm.gossip_peer_state();

        Arc::new(Mutex::new(Self { swarm }))
    }
}

pub fn start_node_service(
    node_service: Arc<Mutex<NodeService>>,
    mut peer_service_out_receiver: mpsc::UnboundedReceiver<OutPeerNotification>,
    peer_service_in_sender: mpsc::UnboundedSender<InPeerNotification>,
) -> oneshot::Sender<()> {
    let (exit_sender, exit_receiver) = oneshot::channel();

    task::spawn(futures::future::select(
        futures::future::poll_fn(move |cx: &mut Context| -> Poll<()> {
            loop {
                match peer_service_out_receiver.poll_next_unpin(cx) {
                    Poll::Ready(Some(event)) => match event {
                        OutPeerNotification::PeerConnected { peer_id } => node_service
                            .lock()
                            .unwrap()
                            .swarm
                            .add_connected_peer(peer_id),
                        OutPeerNotification::PeerDisconnected { peer_id } => node_service
                            .lock()
                            .unwrap()
                            .swarm
                            .remove_connected_peer(peer_id),
                        OutPeerNotification::Relay {
                            src_id,
                            dst_id,
                            data,
                        } => node_service.lock().unwrap().swarm.relay(RelayEvent {
                            src_id: src_id.into_bytes(),
                            dst_id: dst_id.into_bytes(),
                            data,
                        }),
                        OutPeerNotification::GetNetworkState { src_id } => {
                            let service = node_service.lock().unwrap();
                            let network_state = service.swarm.network_state();
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
                    },
                    Poll::Pending => break,
                    Poll::Ready(None) => {
                        // TODO: propagate error
                        break;
                    }
                }
            }

            loop {
                match node_service.lock().unwrap().swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(e)) => {
                        trace!("node_service/poll: sending {:?} to node_service", e);

                        peer_service_in_sender
                            .unbounded_send(InPeerNotification::Relay {
                                src_id: PeerId::from_bytes(e.src_id).unwrap(),
                                dst_id: PeerId::from_bytes(e.dst_id).unwrap(),
                                data: e.data,
                            })
                            .unwrap();
                    }
                    Poll::Ready(None) => unreachable!("stream never ends"),
                    Poll::Pending => break,
                }
            }

            Poll::Pending
        }),
        exit_receiver,
    ));

    exit_sender
}
