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
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};
use serde_json::error;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::sync::mpsc;

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
    peer_channel_out: mpsc::UnboundedReceiver<OutPeerNotification>,
    peer_channel_in: mpsc::UnboundedSender<InPeerNotification>,
    executor: &TaskExecutor,
) -> error::Result<tokio::sync::oneshot::Sender<()>> {
    let (exit_sender, exit_receiver) = tokio::sync::oneshot::channel();

    executor.spawn(
        node_service_executor(node_service.clone(), peer_channel_out, peer_channel_in)
            .select(exit_receiver.then(|_| Ok(())))
            .then(move |_| {
                trace!("peer_service/service: shutting down by external cmd");

                // notify network that this node just has been shutdown
                // TODO: hardering
                node_service.lock().unwrap().swarm.exit();
                Ok(())
            }),
    );

    Ok(exit_sender)
}

fn node_service_executor(
    peer_service: Arc<Mutex<NodeService>>,
    mut peer_service_out: mpsc::UnboundedReceiver<OutPeerNotification>,
    mut peer_service_in: mpsc::UnboundedSender<InPeerNotification>,
) -> impl futures::Future<Item = (), Error = ()> {
    futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match peer_service_out.poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    OutPeerNotification::PeerConnected { peer_id } => peer_service
                        .lock()
                        .unwrap()
                        .swarm
                        .add_connected_peer(peer_id),
                    OutPeerNotification::PeerDisconnected { peer_id } => peer_service
                        .lock()
                        .unwrap()
                        .swarm
                        .remove_connected_peer(peer_id),
                    OutPeerNotification::Relay {
                        src_id,
                        dst_id,
                        data,
                    } => peer_service.lock().unwrap().swarm.relay(RelayEvent {
                        src_id: src_id.into_bytes(),
                        dst_id: dst_id.into_bytes(),
                        data,
                    }),
                    OutPeerNotification::GetNetworkState { src_id } => {
                        let service = peer_service.lock().unwrap();
                        let network_state = service.swarm.network_state();
                        let network_state = network_state
                            .iter()
                            .map(|v| v.0.clone())
                            .collect::<Vec<PeerId>>();
                        peer_service_in
                            .try_send(InPeerNotification::NetworkState {
                                dst_id: src_id,
                                state: network_state,
                            })
                            .expect("Failed during send event to the node service");
                    }
                },
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(None)) => {
                    // TODO: propagate error
                    break;
                }
                Err(_) => {
                    // TODO: propagate error
                    break;
                }
            }
        }

        loop {
            match peer_service.lock().unwrap().swarm.poll() {
                Ok(Async::Ready(Some(_))) => {}
                Ok(Async::Ready(None)) => unreachable!("stream never ends"),
                Ok(Async::NotReady) => break,
                Err(_) => break,
            }
        }

        if let Some(e) = peer_service.lock().unwrap().swarm.pop_peer_relay_message() {
            trace!("node_service/poll: sending {:?} to node_service", e);

            peer_service_in
                .try_send(InPeerNotification::Relay {
                    src_id: PeerId::from_bytes(e.src_id).unwrap(),
                    dst_id: PeerId::from_bytes(e.dst_id).unwrap(),
                    data: e.data,
                })
                .unwrap();
        }

        Ok(Async::NotReady)
    })
}
