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

use super::behaviour::NetworkBehaviour;

use config_utils::to_peer_id;
use fluence_libp2p::{build_transport, types::OneshotOutlet};
use server_config::{
    default_air_interpreter_path, ListenConfig, NetworkConfig, NodeConfig, ServicesConfig,
};
use trust_graph::TrustGraph;

use anyhow::Context;
use async_std::sync::Mutex;
use async_std::task;
use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet};
use futures::{channel::oneshot, future::BoxFuture, select, stream::StreamExt, FutureExt, SinkExt};
use libp2p::swarm::{AddressScore, ExpandedSwarm};
use libp2p::{
    core::{multiaddr::Protocol, Multiaddr},
    identity::ed25519::Keypair,
    PeerId, Swarm, TransportError,
};
use particle_protocol::Particle;
use prometheus::Registry;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;

pub mod unlocks {
    use async_std::sync::Mutex;
    use futures::Future;
    use std::ops::DerefMut;

    pub async fn unlock_f<T, R, F: Future<Output = R>>(
        m: &Mutex<T>,
        f: impl FnOnce(&mut T) -> F,
    ) -> R {
        unlock(m, f).await.await
    }

    pub async fn unlock<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = m.lock().await;
        let result = f(guard.deref_mut());
        drop(guard);
        result
    }
}

use crate::metrics::start_metrics_endpoint;
use crate::network_api::NetworkApi;
use aquamarine::{AquamarineApi, AquamarineBackend, StepperEffects, VmPoolConfig};
use async_std::task::JoinHandle;
use futures::channel::mpsc;
use futures::channel::oneshot::Canceled;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use particle_closures::{HostClosures, NodeInfo};
use std::time::Duration;

// TODO: documentation
pub struct Node {
    pub network_api: NetworkApi,
    pub swarm: Swarm<NetworkBehaviour>,
    stepper_pool: AquamarineBackend,
    stepper_pool_api: AquamarineApi,
    local_peer_id: PeerId,
    registry: Option<Registry>,
    metrics_listen_addr: SocketAddr,
    bootstrap_nodes: Vec<Multiaddr>,
}

impl Node {
    pub fn new(key_pair: Keypair, config: NodeConfig) -> anyhow::Result<Box<Self>> {
        let transport = {
            let key_pair = libp2p::identity::Keypair::Ed25519(key_pair.clone());
            build_transport(key_pair, config.socket_timeout)
        };
        let trust_graph = TrustGraph::new(config.root_weights());

        let local_peer_id = to_peer_id(&key_pair);

        let pool_config = VmPoolConfig::new(
            local_peer_id,
            config.stepper_base_dir.clone(),
            config.air_interpreter_path.clone(),
            config.stepper_pool_size,
        )
        .expect("create vm pool config");

        let services_config = ServicesConfig::new(
            local_peer_id,
            config.services_base_dir.clone(),
            config.services_envs.clone(),
        )
        .expect("create services config");

        let registry = Registry::new();
        let network_config =
            NetworkConfig::new(trust_graph, Some(registry.clone()), key_pair, &config);

        Self::with(
            local_peer_id,
            transport,
            services_config,
            pool_config,
            network_config,
            config.external_addresses(),
            registry.into(),
            config.metrics_listen_addr(),
            config.bootstrap_nodes,
        )
    }

    pub fn with(
        local_peer_id: PeerId,
        transport: Boxed<(PeerId, StreamMuxerBox)>,
        services_config: ServicesConfig,
        pool_config: VmPoolConfig,
        network_config: NetworkConfig,
        external_addresses: Vec<Multiaddr>,
        registry: Option<Registry>,
        metrics_listen_addr: SocketAddr,
        bootstrap_nodes: Vec<Multiaddr>,
        script_storage_interval: Duration,
    ) -> anyhow::Result<Box<Self>> {
        log::info!("server peer id = {}", local_peer_id);

        let (mut swarm, network_api) = {
            let (behaviour, network_api) = NetworkBehaviour::new(network_config)
                .context("failed to crate NetworkBehaviour")?;
            let swarm = Swarm::new(transport, behaviour, local_peer_id);

            (swarm, network_api)
        };

        // Add external addresses to Swarm
        external_addresses.iter().cloned().for_each(|addr| {
            Swarm::add_external_address(&mut swarm, addr, AddressScore::Finite(1));
        });

        let node_info = NodeInfo { external_addresses };
        let host_closures = HostClosures::new(
            network_api.connectivity(),
            node_info,
            services_config,
            script_storage_interval,
        );

        let (stepper_pool, stepper_pool_api) =
            AquamarineBackend::new(pool_config, host_closures.descriptor());

        let node_service = Self {
            network_api,
            swarm,
            stepper_pool,
            stepper_pool_api,
            local_peer_id,
            registry,
            metrics_listen_addr,
            bootstrap_nodes,
        };

        Ok(Box::new(node_service))
    }

    /// Starts node service
    pub fn start(mut self: Box<Self>) -> OneshotOutlet<()> {
        let (exit_outlet, exit_inlet) = oneshot::channel();
        let mut exit_inlet = exit_inlet.into_stream().fuse();

        task::spawn(async move {
            let mut metrics = if let Some(registry) = self.registry {
                start_metrics_endpoint(registry, self.metrics_listen_addr)
            } else {
                futures::future::ready(Ok(())).boxed()
            }
            .fuse();

            let pool = self.stepper_pool.start();
            let network = {
                let pool_api = self.stepper_pool_api;
                let bootstrap_nodes = self.bootstrap_nodes.into_iter().collect();
                self.network_api.start(pool_api, bootstrap_nodes)
            };
            loop {
                select!(
                    _ = self.swarm.select_next_some() => {},
                    _ = metrics => {},
                    event = exit_inlet.next() => {
                        // Ignore Err and None – if exit_outlet is dropped, we'll run forever!
                        if let Some(Ok(_)) = event {
                            break
                        }
                    }
                )
            }

            log::info!("Stopping node");
            network.cancel().await;
            pool.cancel().await;
        });

        exit_outlet
    }

    /// Starts node service listener.
    #[inline]
    pub fn listen(
        &mut self,
        addrs: impl Into<Vec<Multiaddr>>,
    ) -> Result<(), TransportError<io::Error>> {
        let addrs = addrs.into();
        log::info!("Fluence listening on {:?}", addrs);

        for addr in addrs {
            Swarm::listen_on(&mut self.swarm, addr)?;
        }
        Ok(())
    }
}

pub fn write_default_air_interpreter() -> anyhow::Result<()> {
    use air_interpreter_wasm::INTERPRETER_WASM;
    use std::fs::write;

    let destination = default_air_interpreter_path();
    write(&destination, INTERPRETER_WASM).context(format!(
        "writing default INTERPRETER_WASM to {:?}",
        destination
    ))
}

#[cfg(test)]
mod tests {
    use crate::node::write_default_air_interpreter;
    use crate::Node;
    use ctrlc_adapter::block_until_ctrlc;
    use fluence_libp2p::RandomPeerId;
    use libp2p::core::connection::ConnectionId;
    use libp2p::core::Multiaddr;
    use libp2p::identity::ed25519::Keypair;
    use libp2p::swarm::NetworkBehaviour;
    use libp2p::Swarm;
    use maplit::hashmap;
    use particle_protocol::{HandlerMessage, Particle};
    use serde_json::json;
    use server_config::{deserialize_config, NodeConfig};
    use std::path::PathBuf;
    use test_utils::ConnectedClient;

    #[test]
    fn run_node() {
        write_default_air_interpreter().unwrap();

        let keypair = Keypair::generate();

        let config = std::fs::read("../deploy/Config.default.toml").expect("find default config");
        let mut config = deserialize_config(<_>::default(), config).expect("deserialize config");
        config.server.stepper_pool_size = 1;
        let mut node = Node::new(keypair, config.server).unwrap();

        let listening_address: Multiaddr = "/ip4/127.0.0.1/tcp/7777".parse().unwrap();
        node.listen(vec![listening_address.clone()]).unwrap();
        Box::new(node).start();

        let mut client = ConnectedClient::connect_to(listening_address).expect("connect client");
        println!("client: {}", client.peer_id);
        let data = hashmap! {
            "name" => json!("folex"),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
                (seq
                    (call relay ("op" "identity") [] void[])
                    (call client ("return" "") [name] void[])
                )
            "#,
            data.clone(),
        );
        let response = client.receive_args();
        println!("got response!: {:#?}", response);

        // block_until_ctrlc();
    }
}
