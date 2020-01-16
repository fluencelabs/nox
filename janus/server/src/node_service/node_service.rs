/*
 * Copyright 2020 Fluence Labs Limited
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
use crate::node_service::behaviour::NodeServiceBehaviour;
use crate::node_service::transport::NodeServiceTransport;
use crate::node_service::{
    events::{InNodeServiceEvent, OutNodeServiceEvent},
    transport::build_transport,
};
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio::runtime::TaskExecutor;
use tokio::sync::mpsc;

pub struct NodeService {
    pub swarm:
        Box<Swarm<NodeServiceTransport, NodeServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

pub struct NodeServiceDescriptor {
    pub exit_sender: tokio::sync::oneshot::Sender<()>,
    pub node_channel_out: mpsc::UnboundedReceiver<OutNodeServiceEvent>,
    pub node_channel_in: mpsc::UnboundedSender<InNodeServiceEvent>,
}

impl NodeService {
    pub fn new(config: NodeServiceConfig) -> Arc<Mutex<Self>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("node service is starting with peer id = {}", local_peer_id);

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let behaviour = NodeServiceBehaviour::new(&local_peer_id, local_key.public());

            Box::new(Swarm::new(transport, behaviour, local_peer_id))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));
        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        Arc::new(Mutex::new(Self { swarm }))
    }
}

pub fn start_node_service(
    node_service: Arc<Mutex<NodeService>>,
    executor: &TaskExecutor,
) -> Result<NodeServiceDescriptor, ()> {
    let (exit_sender, exit_receiver) = tokio::sync::oneshot::channel();
    let (channel_in_1, channel_out_1) = tokio::sync::mpsc::unbounded_channel();
    let (channel_in_2, channel_out_2) = tokio::sync::mpsc::unbounded_channel();

    executor.spawn(
        node_service_executor(node_service.clone(), channel_out_1, channel_in_2)
            .select(exit_receiver.then(|_| Ok(())))
            .then(move |_| {
                // TODO: log

                // notify network that this node just has been shutdown
                // TODO: hardering
                node_service.lock().unwrap().swarm.exit();
                Ok(())
            }),
    );

    Ok(NodeServiceDescriptor {
        exit_sender,
        node_channel_in: channel_in_1,
        node_channel_out: channel_out_2,
    })
}

fn node_service_executor(
    node_service: Arc<Mutex<NodeService>>,
    mut node_service_in: mpsc::UnboundedReceiver<InNodeServiceEvent>,
    mut node_service_out: mpsc::UnboundedSender<OutNodeServiceEvent>,
) -> impl futures::Future<Item = (), Error = ()> {
    futures::future::poll_fn(move || -> Result<_, ()> {
        let mut node_service = node_service
            .lock()
            .expect("node_service couldn't be unlocked");

        loop {
            match node_service_in.poll() {
                Ok(Async::Ready(Some(e))) => match e {
                    InNodeServiceEvent::Relay { src, dst, data } => node_service
                        .swarm
                        .node_connect_protocol
                        .relay_message(src, dst, data),
                    InNodeServiceEvent::NetworkState { dst, state } => node_service
                        .swarm
                        .node_connect_protocol
                        .send_network_state(dst, state),
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
            match node_service.swarm.poll() {
                Ok(Async::Ready(Some(e))) => {
                    trace!("node_service/poll: received {:?} event", e);
                }
                Ok(Async::Ready(None)) => unreachable!("stream never ends"),
                Ok(Async::NotReady) => break,
                Err(_) => break,
            }
        }

        if let Some(e) = node_service.swarm.pop_out_node_event() {
            trace!("node_service/poll: sending {:?} to peer_service", e);

            node_service_out.try_send(e).unwrap();
        }

        Ok(Async::NotReady)
    })
}
