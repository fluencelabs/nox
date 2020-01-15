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

use crate::config::PeerServiceConfig;
use crate::node_service::events::{InNodeServiceEvent, OutNodeServiceEvent};
use crate::peer_service::p2p::{
    behaviour::PeerServiceBehaviour, transport::build_transport, transport::PeerServiceTransport,
};
use crate::peer_service::relay::message::RelayMessage;
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use parity_multiaddr::{Multiaddr, Protocol};
use serde_json::error;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::sync::mpsc;

pub struct PeerService {
    pub swarm:
        Box<Swarm<PeerServiceTransport, PeerServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl PeerService {
    pub fn new(config: PeerServiceConfig) -> Arc<Mutex<Self>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let churn_topic = config.churn_topic;
            let behaviour =
                PeerServiceBehaviour::new(local_peer_id.clone(), local_key.public(), churn_topic);

            Box::new(Swarm::new(transport, behaviour, local_peer_id.clone()))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        swarm.gossip_peer_state();

        Arc::new(Mutex::new(Self { swarm }))
    }
}

pub fn start_peer_service(
    peer_service: Arc<Mutex<PeerService>>,
    node_channel_out: mpsc::UnboundedReceiver<OutNodeServiceEvent>,
    node_channel_in: mpsc::UnboundedSender<InNodeServiceEvent>,
    executor: &TaskExecutor,
) -> error::Result<tokio::sync::oneshot::Sender<()>> {
    let (exit_sender, exit_receiver) = tokio::sync::oneshot::channel();

    executor.spawn(
        peer_service_executor(peer_service.clone(), node_channel_out, node_channel_in)
            .select(exit_receiver.then(|_| Ok(())))
            .then(move |_| {
                // TODO: log

                // notify network that this node just has been shutdown
                // TODO: hardering
                peer_service.lock().unwrap().swarm.exit();
                Ok(())
            }),
    );

    Ok(exit_sender)
}

fn peer_service_executor(
    peer_service: Arc<Mutex<PeerService>>,
    mut node_service_out: mpsc::UnboundedReceiver<OutNodeServiceEvent>,
    mut node_service_in: mpsc::UnboundedSender<InNodeServiceEvent>,
) -> impl futures::Future<Item = (), Error = ()> {
    futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match node_service_out.poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    OutNodeServiceEvent::NodeConnected { node_id } => peer_service
                        .lock()
                        .unwrap()
                        .swarm
                        .add_connected_node(node_id),
                    OutNodeServiceEvent::NodeDisconnected { node_id } => peer_service
                        .lock()
                        .unwrap()
                        .swarm
                        .remove_connected_node(node_id),
                    OutNodeServiceEvent::Relay { src, dst, data } => peer_service
                        .lock()
                        .unwrap()
                        .swarm
                        .relay
                        .relay(RelayMessage {
                            src: src.into_bytes(),
                            dst: dst.into_bytes(),
                            data,
                        }),
                    OutNodeServiceEvent::GetNetworkState { src } => {
                        let service = peer_service.lock().unwrap();
                        let network_state = service.swarm.relay.network_state();
                        let network_state = network_state
                            .iter()
                            .map(|v| v.0.clone())
                            .collect::<Vec<PeerId>>();
                        node_service_in
                            .try_send(InNodeServiceEvent::NetworkState {
                                dst: src,
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

        if let Some(e) = peer_service
            .lock()
            .unwrap()
            .swarm
            .nodes_messages
            .pop_front()
        {
            node_service_in.try_send(InNodeServiceEvent::Relay {
                src: PeerId::from_bytes(e.src).unwrap(),
                dst: PeerId::from_bytes(e.dst).unwrap(),
                data: e.data,
            }).unwrap();
        }

        Ok(Async::NotReady)
    })
}
