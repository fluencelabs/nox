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

use std::process::exit;
use std::sync::Arc;
use std::{io, net::SocketAddr};

use ccp_rpc_client::CCPRpcHttpClient;
use eyre::WrapErr;
use fluence_keypair::KeyPair;
use futures::future::OptionFuture;
use futures::{stream::StreamExt, FutureExt};
use jsonrpsee::ws_client::WsClientBuilder;
use libp2p::swarm::SwarmEvent;
use libp2p::SwarmBuilder;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, Multiaddr},
    identity::Keypair,
    PeerId, Swarm, TransportError,
};
use libp2p_connection_limits::ConnectionLimits;
use libp2p_metrics::{Metrics, Recorder};
use prometheus_client::registry::Registry;
use tokio::sync::{mpsc, oneshot};
use tokio::task;
use tracing::Instrument;

use aquamarine::{
    AquaRuntime, AquamarineApi, AquamarineApiError, AquamarineBackend, DataStoreConfig,
    RemoteRoutingEffects, VmPoolConfig,
};
use chain_connector::ChainConnector;
use chain_listener::ChainListener;
use config_utils::to_peer_id;
use connection_pool::ConnectionPoolT;
use core_manager::CoreManager;
use fluence_libp2p::build_transport;
use health::HealthCheckRegistry;
use particle_builtins::{Builtins, CustomService, NodeInfo};
use particle_execution::ParticleFunctionStatic;
use particle_protocol::ExtendedParticle;
use peer_metrics::{
    ConnectionPoolMetrics, ConnectivityMetrics, ParticleExecutorMetrics, ServicesMetrics,
    ServicesMetricsBackend, SpellMetrics, VmPoolMetrics,
};
use server_config::system_services_config::ServiceKey;
use server_config::{NetworkConfig, ResolvedConfig, ServicesConfig};
use sorcerer::Sorcerer;
use spell_event_bus::api::{PeerEvent, SpellEventBusApi, TriggerEvent};
use spell_event_bus::bus::SpellEventBus;
use system_services::{Deployer, SystemServiceDistros};
use workers::{KeyStorage, PeerScopes, Workers};

use crate::behaviour::FluenceNetworkBehaviourEvent;
use crate::builtins::make_peer_builtin;
use crate::dispatcher::Dispatcher;
use crate::effectors::Effectors;
use crate::http::start_http_endpoint;
use crate::metrics::TokioCollector;
use crate::{Connectivity, Versions};

use super::behaviour::FluenceNetworkBehaviour;

// TODO: documentation
pub struct Node<RT: AquaRuntime> {
    particle_stream: mpsc::Receiver<ExtendedParticle>,
    effects_stream: mpsc::Receiver<Result<RemoteRoutingEffects, AquamarineApiError>>,
    pub swarm: Swarm<FluenceNetworkBehaviour>,

    pub connectivity: Connectivity,
    pub aquamarine_api: AquamarineApi,
    pub dispatcher: Dispatcher,
    aquamarine_backend: AquamarineBackend<RT, Arc<Builtins<Connectivity>>>,
    system_service_deployer: Deployer,

    spell_event_bus_api: SpellEventBusApi,
    spell_event_bus: SpellEventBus,
    spell_events_receiver: mpsc::UnboundedReceiver<TriggerEvent>,
    sorcerer: Sorcerer,

    metrics_registry: Option<Registry>,
    health_registry: Option<HealthCheckRegistry>,
    libp2p_metrics: Option<Arc<Metrics>>,
    services_metrics_backend: ServicesMetricsBackend,

    http_listen_addr: Option<SocketAddr>,

    pub builtins_management_peer_id: PeerId,

    pub scope: PeerScopes,

    allow_local_addresses: bool,
    versions: Versions,

    pub chain_listener: Option<ChainListener>,

    workers: Arc<Workers>,
}

async fn setup_listener(
    connector: Option<Arc<ChainConnector>>,
    config: &ResolvedConfig,
    core_manager: Arc<CoreManager>,
) -> eyre::Result<Option<ChainListener>> {
    if let (Some(connector), Some(chain_config), Some(listener_config)) = (
        connector,
        config.chain_config.clone(),
        config.chain_listener_config.clone(),
    ) {
        let ccp_client = if let Some(ccp_endpoint) = listener_config.ccp_endpoint.clone() {
            let ccp_client = CCPRpcHttpClient::new(ccp_endpoint.clone())
                .await
                .map_err(|err| {
                    log::error!("Error connecting to CCP {ccp_endpoint}, error: {err}");
                    err
                })?;

            Some(ccp_client)
        } else {
            None
        };

        let cc_events_dir = config.dir_config.cc_events_dir.clone();
        let host_id = config.root_key_pair.get_peer_id();
        let ws_client = WsClientBuilder::default()
            .build(&listener_config.ws_endpoint)
            .await
            .map_err(|err| {
                log::error!(
                    "Error connecting to websocket endpoint {}, error: {}",
                    listener_config.ws_endpoint,
                    err
                );
                err
            })?;
        log::info!(
            "Successfully connected to websocket endpoint: {}",
            listener_config.ws_endpoint
        );

        let chain_listener = ChainListener::new(
            chain_config,
            listener_config,
            host_id,
            connector,
            core_manager,
            ws_client,
            ccp_client,
            cc_events_dir,
        );
        Ok(Some(chain_listener))
    } else {
        Ok(None)
    }
}

impl<RT: AquaRuntime> Node<RT> {
    pub async fn new(
        config: ResolvedConfig,
        core_manager: Arc<CoreManager>,
        vm_config: RT::Config,
        data_store_config: DataStoreConfig,
        node_version: &'static str,
        air_version: &'static str,
        system_service_distros: SystemServiceDistros,
    ) -> eyre::Result<Box<Self>> {
        let key_pair: Keypair = config.node_config.root_key_pair.clone().into();
        let transport = config.transport_config.transport;
        let transport =
            build_transport(transport, &key_pair, config.transport_config.socket_timeout);

        let builtins_peer_id = to_peer_id(&config.builtins_key_pair.clone().into());

        let root_key_pair: KeyPair = key_pair.clone().into();

        let key_storage = KeyStorage::from_path(
            config.dir_config.keypairs_base_dir.clone(),
            root_key_pair.clone(),
        )
        .await?;

        let key_storage = Arc::new(key_storage);

        let scopes = PeerScopes::new(
            root_key_pair.get_peer_id(),
            config.management_peer_id,
            builtins_peer_id,
            key_storage.clone(),
        );

        let (workers, worker_events) = Workers::from_path(
            config.dir_config.workers_base_dir.clone(),
            key_storage.clone(),
            core_manager.clone(),
            config.node_config.workers_queue_buffer,
        )
        .await?;

        let workers = Arc::new(workers);

        let services_config = ServicesConfig::new(
            scopes.get_host_peer_id(),
            config.dir_config.services_persistent_dir.clone(),
            config.dir_config.services_ephemeral_dir.clone(),
            config_utils::particles_vault_dir(&config.dir_config.avm_base_dir),
            config.services_envs.clone(),
            config.management_peer_id,
            builtins_peer_id,
            config.node_config.default_service_memory_limit,
            config.node_config.allowed_effectors.clone(),
            config.node_config.dev_mode_config.binaries.clone(),
            config.node_config.dev_mode_config.enable,
        )
        .expect("create services config");

        let mut metrics_registry = if config.metrics_config.metrics_enabled {
            Some(Registry::default())
        } else {
            None
        };

        let mut health_registry = if config.health_config.health_check_enabled {
            Some(HealthCheckRegistry::default())
        } else {
            None
        };

        let libp2p_metrics = metrics_registry.as_mut().map(|r| Arc::new(Metrics::new(r)));
        let connectivity_metrics = metrics_registry.as_mut().map(ConnectivityMetrics::new);
        let connection_pool_metrics = metrics_registry.as_mut().map(ConnectionPoolMetrics::new);
        let plumber_metrics = metrics_registry.as_mut().map(ParticleExecutorMetrics::new);
        let vm_pool_metrics = metrics_registry.as_mut().map(VmPoolMetrics::new);
        let spell_metrics = metrics_registry.as_mut().map(SpellMetrics::new);

        if config.metrics_config.tokio_metrics_enabled {
            if let Some(r) = metrics_registry.as_mut() {
                let r = r.sub_registry_with_prefix("tokio");
                r.register_collector(Box::new(TokioCollector::new()))
            }
        }

        #[allow(deprecated)]
        let connection_limits = ConnectionLimits::default()
            .with_max_pending_incoming(config.node_config.transport_config.max_pending_incoming)
            .with_max_pending_outgoing(config.node_config.transport_config.max_pending_outgoing)
            .with_max_established_incoming(
                config.node_config.transport_config.max_established_incoming,
            )
            .with_max_established_outgoing(
                config.node_config.transport_config.max_established_outgoing,
            )
            .with_max_established_per_peer(
                config.node_config.transport_config.max_established_per_peer,
            )
            .with_max_established(config.node_config.transport_config.max_established);

        let network_config = NetworkConfig::new(
            libp2p_metrics.clone(),
            connectivity_metrics,
            connection_pool_metrics,
            key_pair,
            &config,
            node_version,
            connection_limits,
        );

        let allow_local_addresses = config.allow_local_addresses;

        let (swarm, connectivity, particle_stream) = Self::swarm(
            root_key_pair.clone().into(),
            network_config,
            transport,
            config.external_addresses(),
            health_registry.as_mut(),
            metrics_registry.as_mut(),
        )?;

        let (services_metrics_backend, services_metrics) =
            if let Some(registry) = metrics_registry.as_mut() {
                ServicesMetrics::with_external_backend(
                    config.metrics_config.metrics_timer_resolution,
                    config.metrics_config.max_builtin_metrics_storage_size,
                    registry,
                )
            } else {
                ServicesMetrics::with_simple_backend(
                    config.metrics_config.max_builtin_metrics_storage_size,
                )
            };

        let mut builtins = Self::builtins(
            connectivity.clone(),
            services_config,
            services_metrics,
            key_storage.clone(),
            workers.clone(),
            scopes.clone(),
            health_registry.as_mut(),
            config.system_services.decider.network_api_endpoint.clone(),
        );

        builtins.services.create_persisted_services().await?;

        let builtins = Arc::new(builtins);

        let (effects_out, effects_in) = mpsc::channel(config.node_config.effects_queue_buffer);

        let pool_config =
            VmPoolConfig::new(config.aquavm_pool_size, config.particle_execution_timeout);
        let (aquamarine_backend, aquamarine_api) = AquamarineBackend::new(
            pool_config,
            vm_config,
            data_store_config,
            Arc::clone(&builtins),
            effects_out,
            plumber_metrics,
            vm_pool_metrics,
            health_registry.as_mut(),
            workers.clone(),
            key_storage.clone(),
            scopes.clone(),
            worker_events,
        )?;
        let effectors = Effectors::new(connectivity.clone());
        let dispatcher = {
            let parallelism = config.particle_processor_parallelism;
            Dispatcher::new(
                scopes.get_host_peer_id(),
                aquamarine_api.clone(),
                effectors,
                parallelism,
                metrics_registry.as_mut(),
            )
        };

        let recv_connection_pool_events = connectivity.connection_pool.lifecycle_events();
        let sources = vec![recv_connection_pool_events.map(PeerEvent::from).boxed()];

        let (spell_event_bus, spell_event_bus_api, spell_events_receiver) =
            SpellEventBus::new(spell_metrics.clone(), sources);

        let spell_service_api = spell_service_api::SpellServiceApi::new(builtins.services.clone());
        let (sorcerer, mut custom_service_functions, spell_version) = Sorcerer::new(
            builtins.services.clone(),
            builtins.modules.clone(),
            aquamarine_api.clone(),
            config.clone(),
            spell_event_bus_api.clone(),
            workers.clone(),
            key_storage.clone(),
            scopes.clone(),
            spell_service_api.clone(),
            spell_metrics,
        );

        let allowed_binaries = config
            .allowed_effectors
            .values()
            .flat_map(|v| v.values().cloned().collect::<Vec<String>>())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<_>();
        let node_info = NodeInfo {
            external_addresses: config.external_addresses(),
            node_version: env!("CARGO_PKG_VERSION"),
            air_version: air_interpreter_wasm::VERSION,
            spell_version: spell_version.clone(),
            // TODO: remove
            allowed_binaries,
        };
        if let Some(m) = metrics_registry.as_mut() {
            peer_metrics::add_info_metrics(
                m,
                node_info.node_version.to_string(),
                node_info.air_version.to_string(),
                node_info.spell_version.clone(),
            );
        }
        custom_service_functions.extend_one(make_peer_builtin(node_info));

        let services = builtins.services.clone();
        let modules = builtins.modules.clone();

        let connector = if let Some(chain_config) = config.chain_config.clone() {
            let host_id = scopes.get_host_peer_id();
            let (chain_connector, chain_builtins) =
                ChainConnector::new(chain_config.clone(), host_id).map_err(|err| {
                    log::error!(
                        "Error connecting to http endpoint {}, error: {err}",
                        chain_config.http_endpoint
                    );
                    err
                })?;
            custom_service_functions.extend(chain_builtins.into_iter());
            Some(chain_connector)
        } else {
            if config.system_services.enable.contains(&ServiceKey::Decider) {
                log::error!(
                    "Decider cannot be used without chain connector. Please, specify chain config"
                );
                exit(1);
            }

            None
        };

        custom_service_functions.into_iter().for_each(
            move |(
                service_id,
                CustomService {
                    functions,
                    fallback,
                },
            )| {
                let builtin = builtins.clone();
                let task = async move { builtin.extend(service_id, functions, fallback).await };
                task::Builder::new()
                    .name("Builtin extend")
                    .spawn(task)
                    .expect("Could not spawn task");
            },
        );

        let system_services_deployer = Deployer::new(
            services,
            modules,
            sorcerer.spell_storage.clone(),
            spell_event_bus_api.clone(),
            spell_service_api,
            scopes.get_host_peer_id(),
            builtins_peer_id,
            system_service_distros,
        );

        let versions = Versions::new(
            node_version.to_string(),
            air_version.to_string(),
            spell_version,
            system_services_deployer.versions(),
        );

        let chain_listener = setup_listener(connector, &config, core_manager).await?;

        Ok(Self::with(
            particle_stream,
            effects_in,
            swarm,
            connectivity,
            aquamarine_api,
            dispatcher,
            aquamarine_backend,
            system_services_deployer,
            spell_event_bus_api,
            spell_event_bus,
            spell_events_receiver,
            sorcerer,
            metrics_registry,
            health_registry,
            libp2p_metrics,
            services_metrics_backend,
            config.http_listen_addr(),
            builtins_peer_id,
            scopes,
            allow_local_addresses,
            versions,
            chain_listener,
            workers.clone(),
        ))
    }

    pub fn swarm(
        key_pair: Keypair,
        network_config: NetworkConfig,
        transport: Boxed<(PeerId, StreamMuxerBox)>,
        external_addresses: Vec<Multiaddr>,
        health_registry: Option<&mut HealthCheckRegistry>,
        metrics_registry: Option<&mut Registry>,
    ) -> eyre::Result<(
        Swarm<FluenceNetworkBehaviour>,
        Connectivity,
        mpsc::Receiver<ExtendedParticle>,
    )> {
        let connection_idle_timeout = network_config.connection_idle_timeout;

        let (behaviour, connectivity, particle_stream) =
            FluenceNetworkBehaviour::new(network_config, health_registry);

        let mut swarm = match metrics_registry {
            None => SwarmBuilder::with_existing_identity(key_pair)
                .with_tokio()
                .with_other_transport(|_| transport)?
                .with_behaviour(|_| behaviour)?
                .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(connection_idle_timeout))
                .build(),
            Some(registry) => SwarmBuilder::with_existing_identity(key_pair)
                .with_tokio()
                .with_other_transport(|_| transport)?
                .with_bandwidth_metrics(registry)
                .with_behaviour(|_| behaviour)?
                .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(connection_idle_timeout))
                .build(),
        };
        // Add external addresses to Swarm
        external_addresses.iter().cloned().for_each(|addr| {
            Swarm::add_external_address(&mut swarm, addr);
        });
        Ok((swarm, connectivity, particle_stream))
    }

    pub fn builtins(
        connectivity: Connectivity,
        services_config: ServicesConfig,
        services_metrics: ServicesMetrics,
        key_storage: Arc<KeyStorage>,
        workers: Arc<Workers>,
        scopes: PeerScopes,
        health_registry: Option<&mut HealthCheckRegistry>,
        connector_api_endpoint: String,
    ) -> Builtins<Connectivity> {
        Builtins::new(
            connectivity,
            services_config,
            services_metrics,
            key_storage,
            workers,
            scopes,
            health_registry,
            connector_api_endpoint,
        )
    }
}

pub struct StartedNode {
    pub exit_outlet: oneshot::Sender<()>,
    pub http_listen_addr: Option<SocketAddr>,
}

impl<RT: AquaRuntime> Node<RT> {
    #[allow(clippy::too_many_arguments)]
    pub fn with(
        particle_stream: mpsc::Receiver<ExtendedParticle>,
        effects_stream: mpsc::Receiver<Result<RemoteRoutingEffects, AquamarineApiError>>,
        swarm: Swarm<FluenceNetworkBehaviour>,
        connectivity: Connectivity,
        aquamarine_api: AquamarineApi,
        dispatcher: Dispatcher,
        aquamarine_backend: AquamarineBackend<RT, Arc<Builtins<Connectivity>>>,
        system_service_deployer: Deployer,
        spell_event_bus_api: SpellEventBusApi,
        spell_event_bus: SpellEventBus,
        spell_events_receiver: mpsc::UnboundedReceiver<TriggerEvent>,
        sorcerer: Sorcerer,
        metrics_registry: Option<Registry>,
        health_registry: Option<HealthCheckRegistry>,
        libp2p_metrics: Option<Arc<Metrics>>,
        services_metrics_backend: ServicesMetricsBackend,
        http_listen_addr: Option<SocketAddr>,
        builtins_management_peer_id: PeerId,
        scope: PeerScopes,
        allow_local_addresses: bool,
        versions: Versions,
        chain_listener: Option<ChainListener>,
        workers: Arc<Workers>,
    ) -> Box<Self> {
        let node_service = Self {
            particle_stream,
            effects_stream,
            swarm,

            connectivity,
            aquamarine_api,
            dispatcher,
            aquamarine_backend,
            system_service_deployer,
            spell_event_bus_api,
            spell_event_bus,
            spell_events_receiver,
            sorcerer,

            metrics_registry,
            health_registry,
            libp2p_metrics,
            services_metrics_backend,
            http_listen_addr,
            builtins_management_peer_id,
            scope,
            allow_local_addresses,
            versions,
            chain_listener,
            workers,
        };

        Box::new(node_service)
    }

    /// Starts node service
    #[allow(clippy::boxed_local)] // Mike said it should be boxed
    pub async fn start(self: Box<Self>, peer_id: PeerId) -> eyre::Result<StartedNode> {
        let (exit_outlet, exit_inlet) = oneshot::channel();
        let (http_bind_outlet, http_bind_inlet) = oneshot::channel();

        let particle_stream = self.particle_stream;
        let effects_stream = self.effects_stream;
        let mut swarm = self.swarm;
        let connectivity = self.connectivity;
        let dispatcher = self.dispatcher;
        let aquamarine_backend = self.aquamarine_backend;
        let spell_event_bus = self.spell_event_bus;
        let spell_events_receiver = self.spell_events_receiver;
        let sorcerer = self.sorcerer;
        let metrics_registry = self.metrics_registry;
        let health_registry = self.health_registry;
        let services_metrics_backend = self.services_metrics_backend;
        let http_listen_addr = self.http_listen_addr;
        let task_name = format!("node-{peer_id}");
        let libp2p_metrics = self.libp2p_metrics;
        let allow_local_addresses = self.allow_local_addresses;
        let versions = self.versions;
        let workers = self.workers.clone();
        let chain_listener = self.chain_listener;

        task::Builder::new().name(&task_name.clone()).spawn(async move {
            let mut http_server = if let Some(http_listen_addr) = http_listen_addr {
                tracing::info!("Starting http endpoint at {}", http_listen_addr);
                async move {
                    start_http_endpoint(http_listen_addr, metrics_registry, health_registry, peer_id, versions, http_bind_outlet)
                        .await.expect("Could not start http server");
                }.boxed()
            } else {
                futures::future::pending().boxed()
            };


            let services_metrics_backend = services_metrics_backend.start();
            let spell_event_bus = spell_event_bus.start();
            let sorcerer = sorcerer.start(spell_events_receiver);
            let chain_listener = chain_listener.map(|c| c.start());
            let aquamarine_backend = aquamarine_backend.start();
            let mut connectivity = connectivity.start();
            let mut dispatcher = dispatcher.start(particle_stream, effects_stream);
            let mut exit_inlet = Some(exit_inlet);
            loop {
                let exit_inlet = exit_inlet.as_mut().expect("Could not get exit inlet");
                tokio::select! {
                    Some(e) = swarm.next() => {
                        if let Some(m) = libp2p_metrics.as_ref() { m.record(&e) }
                        if let SwarmEvent::Behaviour(FluenceNetworkBehaviourEvent::Identify(i)) = e {
                            swarm.behaviour_mut().inject_identify_event(i, allow_local_addresses);
                        }
                    },
                    _ = &mut http_server => {},
                    _ = &mut connectivity => {},
                    _ = &mut dispatcher => {},
                    _ = exit_inlet => {
                        log::info!("Exit inlet");
                        break;
                    }
                }
            }

            log::info!("Stopping node");
            if let Some(c) = chain_listener { c.abort() }
            services_metrics_backend.abort();
            spell_event_bus.abort();
            sorcerer.abort();
            dispatcher.cancel().await;
            connectivity.cancel().await;
            aquamarine_backend.abort();
            workers.shutdown();
        }.in_current_span()).expect("Could not spawn task");

        // Note: need to be after the start of the node to be able to subscribe spells
        let deployer = self.system_service_deployer;
        deployer
            .deploy_system_services()
            .await
            .context("deploying system services failed")?;

        self.spell_event_bus_api
            .start_scheduling()
            .await
            .map_err(|e| eyre::eyre!("{e}"))
            .context("running spell event bus failed")?;

        let http_listen_addr = OptionFuture::from(http_listen_addr.map(|_| async {
            let addr = http_bind_inlet.await.expect("http bind sender is dropped");
            addr.listen_addr
        }))
        .await;

        Ok(StartedNode {
            exit_outlet,
            http_listen_addr,
        })
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
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use avm_server::avm_runner::AVMRunner;
    use libp2p::core::Multiaddr;
    use libp2p::PeerId;
    use maplit::hashmap;
    use serde_json::json;

    use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
    use aquamarine::{DataStoreConfig, VmConfig};
    use config_utils::to_peer_id;
    use connected_client::ConnectedClient;
    use core_manager::DummyCoreManager;
    use fs_utils::to_abs_path;
    use server_config::{default_base_dir, load_config_with_args, persistent_dir};
    use system_services::SystemServiceDistros;

    use crate::Node;

    #[tokio::test]
    async fn run_node() {
        log_utils::enable_logs();
        let base_dir = default_base_dir();
        let persistent_dir = persistent_dir(&base_dir);
        fs_utils::create_dir(&base_dir).unwrap();
        fs_utils::create_dir(&persistent_dir).unwrap();
        write_default_air_interpreter(&air_interpreter_path(&persistent_dir)).unwrap();

        let mut config = load_config_with_args(vec![], None)
            .expect("Could not load config")
            .resolve()
            .expect("Could not resolve config");
        config.transport_config.connection_idle_timeout = Duration::from_secs(60);
        config.aquavm_pool_size = 1;
        config.dir_config.spell_base_dir = to_abs_path(PathBuf::from("spell"));
        config.system_services.enable = vec![];
        config.http_config = None;
        let vm_config = VmConfig::new(
            to_peer_id(&config.root_key_pair.clone().into()),
            config.dir_config.air_interpreter_path.clone(),
            None,
            None,
            None,
            None,
            false,
        );
        let data_store_config = DataStoreConfig::new(config.dir_config.avm_base_dir.clone());

        let system_service_distros =
            SystemServiceDistros::default_from(config.system_services.clone())
                .expect("can't create system services");

        let core_manager = Arc::new(DummyCoreManager::default().into());

        let mut node: Box<Node<AVMRunner>> = Node::new(
            config,
            core_manager,
            vm_config,
            data_store_config,
            "some version",
            "some version",
            system_service_distros,
        )
        .await
        .expect("create node");

        let listening_address: Multiaddr = "/ip4/127.0.0.1/tcp/7777".parse().unwrap();
        node.listen(vec![listening_address.clone()]).unwrap();
        let peer_id = PeerId::random();
        let started_node = node.start(peer_id).await.expect("start node");

        let mut client = ConnectedClient::connect_to_with_timeout(
            listening_address,
            Duration::from_secs(10),
            Duration::from_secs(60),
            Some(Duration::from_secs(2 * 60)),
        )
        .await
        .expect("connect client");
        let data = hashmap! {
            "name" => json!("folex"),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client
            .execute_particle(
                r#"
                (seq
                    (call relay ("op" "identity") [])
                    (call client ("return" "") [name])
                )
            "#,
                data.clone(),
            )
            .await
            .unwrap();

        started_node.exit_outlet.send(()).unwrap();
    }
}
