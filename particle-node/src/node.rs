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
use crate::metrics::start_metrics_endpoint;
use crate::network_api::NetworkApi;

use aquamarine::{
    AquaRuntime, AquamarineApi, AquamarineBackend, AquamarineVM, VmConfig, VmPoolConfig,
};
use config_utils::to_peer_id;
use connection_pool::ConnectionPoolApi;
use fluence_libp2p::{
    build_transport,
    types::{OneshotOutlet, Outlet},
};
use particle_closures::{HostClosures, NodeInfo};
use script_storage::{ScriptStorageApi, ScriptStorageBackend, ScriptStorageConfig};
use server_config::{default_air_interpreter_path, NetworkConfig, NodeConfig, ServicesConfig};
use trust_graph::{InMemoryStorage, TrustGraph};

use crate::Connectivity;
use async_std::task;
use eyre::WrapErr;
use futures::{
    channel::{mpsc::unbounded, oneshot},
    select,
    stream::{self, StreamExt},
    FutureExt,
};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, Multiaddr},
    identity::Keypair,
    swarm::AddressScore,
    PeerId, Swarm, TransportError,
};
use prometheus::Registry;
use std::{io, iter::once, net::SocketAddr};

// TODO: documentation
pub struct Node<RT: AquaRuntime> {
    pub network_api: NetworkApi,
    pub swarm: Swarm<NetworkBehaviour>,
    stepper_pool: AquamarineBackend<RT>,
    stepper_pool_api: AquamarineApi,
    #[allow(dead_code)] // useful for debugging
    local_peer_id: PeerId,
    registry: Option<Registry>,
    metrics_listen_addr: SocketAddr,
    bootstrap_nodes: Vec<Multiaddr>,
    particle_failures: Outlet<String>,
    script_storage: ScriptStorageBackend,
}

impl Node<AquamarineVM> {
    pub fn new(key_pair: Keypair, config: NodeConfig) -> Box<Self> {
        let transport = { build_transport(key_pair.clone(), config.socket_timeout) };

        let trust_graph = {
            let storage = InMemoryStorage::new_in_memory(config.root_weights()?);
            TrustGraph::new(storage)
        };

        let local_peer_id = to_peer_id(&key_pair);

        let vm_config = VmConfig::new(
            local_peer_id,
            config.stepper_base_dir.clone(),
            config.air_interpreter_path.clone(),
        )
        .expect("create vm config");

        let pool_config =
            VmPoolConfig::new(config.stepper_pool_size, config.particle_execution_timeout);

        let services_config = ServicesConfig::new(
            local_peer_id,
            config.services_base_dir.clone(),
            config.services_envs.clone(),
            config.management_peer_id,
        )
        .expect("create services config");

        let registry = Registry::new();
        let network_config =
            NetworkConfig::new(trust_graph, Some(registry.clone()), key_pair, &config);

        let (swarm, network_api) = Self::swarm(
            local_peer_id,
            network_config,
            transport,
            config.external_addresses(),
        );

        let (particle_failures_out, particle_failures_in) = unbounded();

        let connectivity = network_api.connectivity();
        let (script_storage_api, script_storage_backend) = {
            let script_storage_config = ScriptStorageConfig {
                timer_resolution: config.script_storage_timer_resolution,
                max_failures: config.script_storage_max_failures,
                particle_ttl: config.script_storage_particle_ttl,
                peer_id: local_peer_id,
            };

            let pool: &ConnectionPoolApi = connectivity.as_ref();
            ScriptStorageBackend::new(pool.clone(), particle_failures_in, script_storage_config)
        };

        let host_closures = Self::host_closures(
            connectivity,
            config.external_addresses(),
            services_config,
            script_storage_api,
        );
        let vm_config = (vm_config, host_closures.descriptor());

        Self::with(
            local_peer_id,
            swarm,
            network_api,
            script_storage_backend,
            vm_config,
            pool_config,
            particle_failures_out,
            registry.into(),
            config.metrics_listen_addr(),
            config.bootstrap_nodes,
        )
    }

    pub fn swarm(
        local_peer_id: PeerId,
        network_config: NetworkConfig,
        transport: Boxed<(PeerId, StreamMuxerBox)>,
        external_addresses: Vec<Multiaddr>,
    ) -> (Swarm<NetworkBehaviour>, NetworkApi) {
        let (behaviour, network_api) = NetworkBehaviour::new(network_config);
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

        // Add external addresses to Swarm
        external_addresses.iter().cloned().for_each(|addr| {
            Swarm::add_external_address(&mut swarm, addr, AddressScore::Finite(1));
        });

        (swarm, network_api)
    }

    pub fn host_closures(
        connectivity: Connectivity,
        external_addresses: Vec<Multiaddr>,
        services_config: ServicesConfig,
        script_storage_api: ScriptStorageApi,
    ) -> HostClosures<Connectivity> {
        let node_info = NodeInfo { external_addresses };

        HostClosures::new(connectivity, script_storage_api, node_info, services_config)
    }
}

impl<RT: AquaRuntime> Node<RT> {
    #[allow(clippy::too_many_arguments)]
    pub fn with(
        local_peer_id: PeerId,
        swarm: Swarm<NetworkBehaviour>,
        network_api: NetworkApi,
        script_storage: ScriptStorageBackend,

        vm_config: RT::Config,
        pool_config: VmPoolConfig,

        particle_failures: Outlet<String>,
        registry: Option<Registry>,
        metrics_listen_addr: SocketAddr,
        bootstrap_nodes: Vec<Multiaddr>,
    ) -> Box<Self> {
        log::info!("server peer id = {}", local_peer_id);

        let (stepper_pool, stepper_pool_api) = AquamarineBackend::new(pool_config, vm_config);

        let node_service = Self {
            network_api,
            swarm,
            stepper_pool,
            stepper_pool_api,
            local_peer_id,
            registry,
            metrics_listen_addr,
            bootstrap_nodes,
            particle_failures,
            script_storage,
        };

        Box::new(node_service)
    }

    /// Starts node service
    pub fn start(self: Box<Self>) -> OneshotOutlet<()> {
        let (exit_outlet, exit_inlet) = oneshot::channel();
        let mut exit_inlet = exit_inlet.into_stream().fuse();

        task::spawn(async move {
            let mut metrics = if let Some(registry) = self.registry {
                start_metrics_endpoint(registry, self.metrics_listen_addr)
            } else {
                futures::future::ready(Ok(())).boxed()
            }
            .fuse();

            let script_storage = self.script_storage.start();
            let pool = self.stepper_pool.start();
            let mut network = {
                let pool_api = self.stepper_pool_api;
                let failures = self.particle_failures;
                let bootstrap_nodes = self.bootstrap_nodes.into_iter().collect();
                self.network_api.start(pool_api, bootstrap_nodes, failures)
            };
            let stopped = stream::iter(once(Err(())));
            let mut swarm = self.swarm.map(|_e| Ok(())).chain(stopped).fuse();

            loop {
                select!(
                    e = swarm.select_next_some() => {
                        if e.is_err() {
                            log::error!("Swarm has terminated");
                            break;
                        }
                    },
                    e = metrics => {
                        if let Err(err) = e {
                            log::warn!("Metrics returned error: {}", err)
                        }
                    },
                    _ = network => {},
                    event = exit_inlet.next() => {
                        // Ignore Err and None – if exit_outlet is dropped, we'll run forever!
                        if let Some(Ok(_)) = event {
                            break
                        }
                    }
                )
            }

            log::info!("Stopping node");
            script_storage.cancel().await;
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

pub fn write_default_air_interpreter() -> eyre::Result<()> {
    use air_interpreter_wasm::INTERPRETER_WASM;
    use std::fs::write;

    let destination = default_air_interpreter_path();
    write(&destination, INTERPRETER_WASM).wrap_err(format!(
        "writing default INTERPRETER_WASM to {:?}",
        destination
    ))
}

#[cfg(test)]
mod tests {
    use crate::node::write_default_air_interpreter;
    use crate::Node;
    use eyre::WrapErr;
    use libp2p::core::Multiaddr;
    use libp2p::identity::Keypair;
    use maplit::hashmap;
    use serde_json::json;
    use server_config::deserialize_config;
    use test_utils::ConnectedClient;

    #[test]
    fn run_node() {
        write_default_air_interpreter().unwrap();

        let keypair = Keypair::generate_ed25519();

        let config = std::fs::read("../deploy/Config.default.toml").expect("find default config");
        let mut config = deserialize_config(<_>::default(), config).expect("deserialize config");
        config.server.stepper_pool_size = 1;
        let mut node = Node::new(keypair, config.server);

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
        let response = client.receive_args().wrap_err("receive args").unwrap();
        println!("got response!: {:#?}", response);
    }
}
