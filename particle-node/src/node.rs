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

use std::{io, iter::once, net::SocketAddr};

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
use trust_graph::InMemoryStorage;

use aquamarine::{
    AquaRuntime, AquamarineApi, AquamarineBackend, DataStoreError, Observation, VmConfig,
    VmPoolConfig, AVM,
};
use builtins_deployer::BuiltinsDeployer;
use config_utils::to_peer_id;
use connection_pool::ConnectionPoolApi;
use fluence_libp2p::types::{BackPressuredInlet, Inlet};
use fluence_libp2p::{
    build_transport,
    types::{OneshotOutlet, Outlet},
};
use particle_closures::{HostFunctions, NodeInfo, ParticleFunctions, ParticleFunctionsApi};
use particle_protocol::Particle;
use script_storage::{ScriptStorageApi, ScriptStorageBackend, ScriptStorageConfig};
use server_config::{NetworkConfig, ResolvedConfig, ServicesConfig};

use crate::dispatcher::Dispatcher;
use crate::effectors::Effectors;
use crate::metrics::start_metrics_endpoint;
use crate::Connectivity;

use super::behaviour::NetworkBehaviour;

type TrustGraph = trust_graph::TrustGraph<InMemoryStorage>;

// TODO: documentation
pub struct Node<RT: AquaRuntime> {
    particle_stream: BackPressuredInlet<Particle>,
    observation_stream: Inlet<Observation>,
    pub swarm: Swarm<NetworkBehaviour>,

    pub connectivity: Connectivity,
    pub dispatcher: Dispatcher,
    aquavm_pool: AquamarineBackend<RT>,
    script_storage: ScriptStorageBackend,
    builtins_deployer: BuiltinsDeployer,
    pub particle_functions: ParticleFunctionsApi,

    registry: Option<Registry>,
    metrics_listen_addr: SocketAddr,

    pub local_peer_id: PeerId,
    // TODO: remove?
    pub builtins_management_peer_id: PeerId,
}

impl<RT: AquaRuntime> Node<RT> {
    pub fn new(
        config: ResolvedConfig,
        vm_config: RT::Config,
        node_version: &'static str,
    ) -> eyre::Result<Box<Self>> {
        let key_pair: Keypair = config.node_config.root_key_pair.clone().into();
        let local_peer_id = to_peer_id(&key_pair);
        let transport = config.transport_config.transport;
        let transport = build_transport(
            transport,
            key_pair.clone(),
            config.transport_config.socket_timeout,
        );

        let builtins_peer_id = to_peer_id(&config.builtins_key_pair.clone().into());

        let trust_graph = {
            let storage = InMemoryStorage::new_in_memory(config.root_weights()?);
            TrustGraph::new(storage)
        };

        let services_config = ServicesConfig::new(
            local_peer_id,
            config.dir_config.services_base_dir.clone(),
            config_utils::particles_vault_dir(&config.dir_config.avm_base_dir),
            config.services_envs.clone(),
            config.management_peer_id,
            builtins_peer_id,
        )
        .expect("create services config");

        let metrics_registry = if config.prometheus_config.prometheus_enabled {
            Some(Registry::new())
        } else {
            None
        };
        let network_config =
            NetworkConfig::new(metrics_registry.clone(), key_pair, &config, node_version);

        let (swarm, connectivity, particle_stream) = Self::swarm(
            local_peer_id,
            network_config,
            transport,
            config.external_addresses(),
            trust_graph,
        );

        let (particle_failures_out, particle_failures_in) = unbounded();

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

        let host_functions = Self::host_functions(
            connectivity.clone(),
            config.external_addresses(),
            services_config,
            script_storage_api,
        );
        let (func_inlet, func_outlet) = unbounded();
        let particle_functions = ParticleFunctions::new(func_outlet);
        let particle_functions_api = ParticleFunctionsApi::new(func_inlet);

        let pool_config =
            VmPoolConfig::new(config.aquavm_pool_size, config.particle_execution_timeout);
        let (aquavm_pool, aquamarine_api) = AquamarineBackend::new(pool_config, vm_config);
        let (effectors, observation_stream) =
            Effectors::new(connectivity.clone(), host_functions, particle_functions);
        let dispatcher = {
            let failures = particle_failures_out;
            let parallelism = config.particle_processor_parallelism;
            let timeout = config.particle_processing_timeout;
            Dispatcher::new(
                local_peer_id,
                aquamarine_api.clone(),
                effectors,
                failures,
                parallelism,
                timeout,
            )
        };

        let builtins_deployer = BuiltinsDeployer::new(
            builtins_peer_id,
            local_peer_id,
            aquamarine_api,
            config.dir_config.builtins_base_dir.clone(),
            config.node_config.autodeploy_particle_ttl,
            config.node_config.force_builtins_redeploy,
            config.node_config.autodeploy_retry_attempts,
        );

        Ok(Self::with(
            particle_stream,
            observation_stream,
            swarm,
            connectivity,
            dispatcher,
            aquavm_pool,
            script_storage_backend,
            builtins_deployer,
            metrics_registry,
            config.metrics_listen_addr(),
            local_peer_id,
            builtins_peer_id,
        ))
    }

    pub fn swarm(
        local_peer_id: PeerId,
        network_config: NetworkConfig,
        transport: Boxed<(PeerId, StreamMuxerBox)>,
        external_addresses: Vec<Multiaddr>,
        trust_graph: TrustGraph,
    ) -> (
        Swarm<NetworkBehaviour>,
        Connectivity,
        BackPressuredInlet<Particle>,
    ) {
        let (behaviour, connectivity, particle_stream) =
            NetworkBehaviour::new(network_config, trust_graph);
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

        // Add external addresses to Swarm
        external_addresses.iter().cloned().for_each(|addr| {
            Swarm::add_external_address(&mut swarm, addr, AddressScore::Finite(1));
        });

        (swarm, connectivity, particle_stream)
    }

    pub fn host_functions(
        connectivity: Connectivity,
        external_addresses: Vec<Multiaddr>,
        services_config: ServicesConfig,
        script_storage_api: ScriptStorageApi,
    ) -> HostFunctions<Connectivity> {
        let node_info = NodeInfo {
            external_addresses,
            node_version: env!("CARGO_PKG_VERSION"),
            air_version: air_interpreter_wasm::VERSION,
        };

        HostFunctions::new(connectivity, script_storage_api, node_info, services_config)
    }
}

impl<RT: AquaRuntime> Node<RT> {
    #[allow(clippy::too_many_arguments)]
    pub fn with(
        particle_stream: BackPressuredInlet<Particle>,
        observation_stream: Inlet<Observation>,
        swarm: Swarm<NetworkBehaviour>,

        connectivity: Connectivity,
        dispatcher: Dispatcher,
        aquavm_pool: AquamarineBackend<RT>,
        script_storage: ScriptStorageBackend,
        builtins_deployer: BuiltinsDeployer,
        particle_functions: ParticleFunctionsApi,

        registry: Option<Registry>,
        metrics_listen_addr: SocketAddr,

        local_peer_id: PeerId,
        builtins_management_peer_id: PeerId,
    ) -> Box<Self> {
        let node_service = Self {
            particle_stream,
            observation_stream,
            swarm,

            connectivity,
            dispatcher,
            aquavm_pool,
            script_storage,
            builtins_deployer,
            particle_functions,

            registry,
            metrics_listen_addr,

            local_peer_id,
            builtins_management_peer_id,
        };

        Box::new(node_service)
    }

    /// Starts node service
    pub fn start(self: Box<Self>) -> eyre::Result<OneshotOutlet<()>> {
        let (exit_outlet, exit_inlet) = oneshot::channel();
        let mut exit_inlet = exit_inlet.into_stream().fuse();

        let particle_stream = self.particle_stream;
        let observation_stream = self.observation_stream;
        let swarm = self.swarm;
        let connectivity = self.connectivity;
        let dispatcher = self.dispatcher;
        let aquavm_pool = self.aquavm_pool;
        let script_storage = self.script_storage;
        let registry = self.registry;
        let metrics_listen_addr = self.metrics_listen_addr;
        let local_peer_id = self.local_peer_id;
        let builtins_management_peer_id = self.builtins_management_peer_id;

        task::spawn(async move {
            let mut metrics = if let Some(registry) = registry {
                start_metrics_endpoint(registry, metrics_listen_addr)
            } else {
                futures::future::ready(Ok(())).boxed()
            }
            .fuse();

            let script_storage = script_storage.start();
            let pool = aquavm_pool.start();
            let mut connectivity = connectivity.start();
            log::info!("will start dispatcher");
            let mut dispatcher = dispatcher.start(particle_stream, observation_stream);
            let stopped = stream::iter(once(Err(())));
            let mut swarm = swarm.map(|_e| Ok(())).chain(stopped).fuse();

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
                    _ = connectivity => {},
                    _ = dispatcher => {},
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
            dispatcher.cancel().await;
            connectivity.cancel().await;
            pool.cancel().await;
        });

        // TODO: make `deploy_builtin_services` async and add to the "select! loop" above
        let mut builtins_deployer = self.builtins_deployer;
        builtins_deployer
            .deploy_builtin_services()
            .wrap_err("builtins deploy failed")?;

        Ok(exit_outlet)
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

#[cfg(test)]
mod tests {
    use eyre::WrapErr;
    use libp2p::core::Multiaddr;
    use maplit::hashmap;
    use serde_json::json;

    use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
    use aquamarine::{VmConfig, AVM};
    use config_utils::to_peer_id;
    use connected_client::ConnectedClient;
    use fluence_libp2p::RandomPeerId;
    use server_config::{default_base_dir, deserialize_config};

    use crate::Node;

    #[test]
    fn run_node() {
        fs_utils::create_dir(default_base_dir()).unwrap();
        write_default_air_interpreter(&air_interpreter_path(&default_base_dir())).unwrap();

        let mut config = deserialize_config(&<_>::default(), &[]).expect("deserialize config");
        config.aquavm_pool_size = 1;
        let vm_config = VmConfig::new(
            to_peer_id(&config.root_key_pair.clone().into()),
            config.dir_config.avm_base_dir.clone(),
            config.dir_config.air_interpreter_path.clone(),
        );
        let mut node: Box<Node<AVM<_>>> =
            Node::new(config, vm_config, "some version").expect("create node");

        let listening_address: Multiaddr = "/ip4/127.0.0.1/tcp/7777".parse().unwrap();
        node.listen(vec![listening_address.clone()]).unwrap();
        node.start().expect("start node");

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
                    (call relay ("op" "identity") [])
                    (call client ("return" "") [name])
                )
            "#,
            data.clone(),
        );
        let response = client.receive_args().wrap_err("receive args").unwrap();
        println!("got response!: {:#?}", response);
    }
}
