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

use crate::config::JanusConfig;
use crate::node_handler::message::NodeEvent;
use crate::p2p::{
    behaviour::JanusBehaviour, transport::build_transport, transport::JanusTransport,
};
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

pub struct JanusService {
    pub swarm: Box<Swarm<JanusTransport, JanusBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl JanusService {
    pub fn new(config: JanusConfig) -> Arc<Mutex<Self>> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let churn_topic = config.churn_topic;
            let behaviour =
                JanusBehaviour::new(local_peer_id.clone(), local_key.public(), churn_topic);

            Box::new(Swarm::new(transport, behaviour, local_peer_id.clone()))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        swarm.gossip_peer_state();

        Arc::new(Mutex::new(JanusService { swarm }))
    }
}

pub fn start_janus(
    janus: Arc<Mutex<JanusService>>,
    node_channel: mpsc::UnboundedReceiver<NodeEvent>,
    executor: &TaskExecutor,
) -> error::Result<tokio::sync::oneshot::Sender<()>> {
    let (exit_sender, exit_receiver) = tokio::sync::oneshot::channel();

    executor.spawn(
        janus_executor(janus.clone(), node_channel)
            .select(exit_receiver.then(|_| Ok(())))
            .then(move |_| {
                // TODO: log

                // notify network that this node just has been shutdown
                janus.lock().unwrap().swarm.exit();
                Ok(())
            }),
    );

    Ok(exit_sender)
}

fn janus_executor(
    janus: Arc<Mutex<JanusService>>,
    mut node_channel: mpsc::UnboundedReceiver<NodeEvent>,
) -> impl futures::Future<Item = (), Error = ()> {
    futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match node_channel.poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    NodeEvent::NodeConnected { peer } => {
                        janus.lock().unwrap().swarm.add_connected_node(peer);
                    }
                    NodeEvent::NodeDisconnected { peer } => {
                        janus.lock().unwrap().swarm.remove_connected_node(peer);
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
            match janus.lock().unwrap().swarm.poll() {
                Ok(Async::Ready(Some(_))) => {}
                Ok(Async::Ready(None)) => unreachable!("stream never ends"),
                Ok(Async::NotReady) => break,
                Err(_) => break,
            }
        }

        Ok(Async::NotReady)
    })
}
