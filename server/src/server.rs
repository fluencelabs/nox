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
use fluence_faas_service::RawModulesConfig;
use futures::future::BoxFuture;
use futures::{channel::oneshot, select, stream::StreamExt, FutureExt};
use libp2p::{
    identity::ed25519::{self, Keypair},
    identity::PublicKey,
    PeerId, Swarm, TransportError,
};
use parity_multiaddr::{Multiaddr, Protocol};
use prometheus::Registry;
use std::io;
use std::net::IpAddr;
use trust_graph::TrustGraph;

// TODO: documentation
pub struct Server {
    swarm: Swarm<ServerBehaviour>,
    config: ServerConfig,
    registry: Registry,
}

impl Server {
    pub fn new(
        key_pair: Keypair,
        server_config: ServerConfig,
        faas_config: RawModulesConfig,
        root_weights: Vec<(ed25519::PublicKey, u32)>,
    ) -> Box<Self> {
        let ServerConfig { socket_timeout, .. } = server_config;

        let local_peer_id = PeerId::from(PublicKey::Ed25519(key_pair.public()));
        log::info!("server peer id = {}", local_peer_id);

        let trust_graph = TrustGraph::new(root_weights);
        let registry = Registry::new();

        let mut swarm = {
            let behaviour = ServerBehaviour::new(
                key_pair.clone(),
                local_peer_id.clone(),
                server_config.external_addresses(),
                trust_graph,
                server_config.bootstrap_nodes.clone(),
                Some(&registry),
                faas_config,
                <_>::default(),
            );
            let key_pair = libp2p::identity::Keypair::Ed25519(key_pair);
            let transport = build_transport(key_pair, socket_timeout);

            Swarm::new(transport, behaviour, local_peer_id)
        };

        // Add external addresses to Swarm
        server_config
            .external_addresses()
            .into_iter()
            .for_each(|addr| Swarm::add_external_address(&mut swarm, addr));

        let node_service = Self {
            swarm,
            config: server_config,
            registry,
        };

        Box::new(node_service)
    }

    /// Starts node service
    pub fn start(mut self: Box<Self>) -> OneshotOutlet<()> {
        let (exit_outlet, exit_inlet) = oneshot::channel();
        let mut exit_inlet = exit_inlet.into_stream().fuse();

        self.listen().expect("Error on starting node listener");
        self.swarm.dial_bootstrap_nodes();

        task::spawn(async move {
            let mut metrics = Self::start_metrics_endpoint(
                self.registry,
                (self.config.listen_ip, self.config.prometheus_port),
            )
            .fuse();
            loop {
                select!(
                    _ = self.swarm.select_next_some() => {},
                    _ = metrics => {},
                    _ = exit_inlet.next() => {
                        break
                    }
                )
            }
        });

        exit_outlet
    }

    pub fn start_metrics_endpoint(
        registry: Registry,
        listen_addr: (IpAddr, u16),
    ) -> BoxFuture<'static, io::Result<()>> {
        use http_types::{Error, StatusCode::InternalServerError};
        use prometheus::{Encoder, TextEncoder};

        let mut app = tide::with_state(registry);
        app.at("/metrics")
            .get(|req: tide::Request<Registry>| async move {
                let mut buffer = vec![];
                let encoder = TextEncoder::new();
                let metric_families = req.state().gather();

                encoder
                    .encode(&metric_families, &mut buffer)
                    .map_err(|err| {
                        let msg = format!("Error encoding prometheus metrics: {:?}", err);
                        log::warn!("{}", msg);
                        Error::from_str(InternalServerError, msg)
                    })?;

                String::from_utf8(buffer).map_err(|err| {
                    let msg = format!("Error encoding prometheus metrics: {:?}", err);
                    log::warn!("{}", msg);
                    Error::from_str(InternalServerError, msg)
                })
            });

        Box::pin(app.listen(listen_addr))
    }

    /// Starts node service listener.
    #[inline]
    fn listen(&mut self) -> Result<(), TransportError<io::Error>> {
        let mut tcp = Multiaddr::from(self.config.listen_ip);
        tcp.push(Protocol::Tcp(self.config.tcp_port));

        let mut ws = Multiaddr::from(self.config.listen_ip);
        ws.push(Protocol::Tcp(self.config.websocket_port));
        ws.push(Protocol::Ws("/".into()));

        log::info!("Fluence listening on {} and {}", tcp, ws);

        Swarm::listen_on(&mut self.swarm, tcp)?;
        Swarm::listen_on(&mut self.swarm, ws)?;
        Ok(())
    }
}
