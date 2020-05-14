/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use super::behaviour::ServerBehaviour;
use crate::config::ServerConfig;
use janus_libp2p::{build_transport, types::OneshotOutlet};

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
                trust_graph,
                config.bootstrap_nodes.clone(),
            );
            let key_pair = libp2p::identity::Keypair::Ed25519(key_pair);
            let transport = build_transport(key_pair, socket_timeout);

            Swarm::new(transport, behaviour, local_peer_id)
        };

        if let Some(external_address) = config.external_address {
            let external_tcp = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(config.tcp_port));
                maddr
            };

            let external_ws = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(config.websocket_port));
                maddr.push(Protocol::Ws("/".into()));
                maddr
            };

            Swarm::add_external_address(&mut swarm, external_tcp);
            Swarm::add_external_address(&mut swarm, external_ws);
        }

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
