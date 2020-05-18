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

use super::behaviour::ServerBehaviour;
use crate::config::ServerConfig;
use fluence_libp2p::{build_transport, types::OneshotOutlet};

use async_std::task;
use futures::channel::oneshot::Receiver;
use futures::{channel::oneshot, select, stream::StreamExt, FutureExt};
use futures_util::future::IntoStream;
use futures_util::stream::Fuse;
use libp2p::{
    identity::ed25519::{self, Keypair},
    identity::PublicKey,
    PeerId, Swarm, TransportError,
};
use parity_multiaddr::{Multiaddr, Protocol};
use std::io;
use trust_graph::TrustGraph;

// TODO: documentation
pub struct Server {
    swarm: Swarm<ServerBehaviour>,
    config: ServerConfig,
}

impl Server {
    pub fn new(
        key_pair: Keypair,
        config: ServerConfig,
        root_weights: Vec<(ed25519::PublicKey, u32)>,
    ) -> Box<Self> {
        let ServerConfig { socket_timeout, .. } = config;

        let local_peer_id = PeerId::from(PublicKey::Ed25519(key_pair.public()));
        log::info!("server peer id = {}", local_peer_id);

        let trust_graph = TrustGraph::new(root_weights);

        let mut swarm = {
            let behaviour = ServerBehaviour::new(
                key_pair.clone(),
                local_peer_id.clone(),
                config.external_addresses(),
                trust_graph,
                config.bootstrap_nodes.clone(),
            );
            let key_pair = libp2p::identity::Keypair::Ed25519(key_pair);
            let transport = build_transport(key_pair, socket_timeout);

            Swarm::new(transport, behaviour, local_peer_id)
        };

        // Add external addresses to Swarm
        config
            .external_addresses()
            .into_iter()
            .for_each(|addr| Swarm::add_external_address(&mut swarm, addr));

        let node_service = Self { swarm, config };

        Box::new(node_service)
    }

    /// Starts node service
    pub fn start(mut self: Box<Self>) -> OneshotOutlet<()> {
        let (exit_sender, exit_receiver) = oneshot::channel();
        let mut exit_receiver: Fuse<IntoStream<Receiver<()>>> = exit_receiver.into_stream().fuse();

        self.listen().expect("Error on starting node listener");
        self.swarm.dial_bootstrap_nodes();

        task::spawn(async move {
            loop {
                select!(
                    _ = self.swarm.select_next_some() => {},
                    _ = exit_receiver.next() => {
                        break
                    }
                )
            }
        });

        exit_sender
    }

    /// Starts node service listener.
    #[inline]
    fn listen(&mut self) -> Result<(), TransportError<io::Error>> {
        let mut listen_addr = Multiaddr::from(self.config.listen_ip);
        listen_addr.push(Protocol::Tcp(self.config.tcp_port));

        let mut ws = Multiaddr::from(self.config.listen_ip);
        ws.push(Protocol::Tcp(self.config.websocket_port));
        ws.push(Protocol::Ws("/".into()));

        Swarm::listen_on(&mut self.swarm, listen_addr)?;
        Swarm::listen_on(&mut self.swarm, ws)?;
        Ok(())
    }
}
