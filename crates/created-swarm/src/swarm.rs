/*
 * Copyright 2024 Fluence DAO
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

use std::collections::HashMap;
use std::convert::identity;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::available_parallelism;
use std::{path::PathBuf, time::Duration};

use derivative::Derivative;
use fluence_keypair::KeyPair;
use futures::{stream, FutureExt, StreamExt};
use libp2p::core::multiaddr::Protocol;
use libp2p::{core::Multiaddr, PeerId};
use serde::Deserialize;

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::{AVMRunner, AquamarineApi, VmConfig};
use aquamarine::{AquaRuntime, DataStoreConfig};
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use cid_utils::Hash;
use core_distributor::{AcquireStrategy, CoreRange, PersistenceTask, PersistentCoreDistributor};
use cpu_utils::HwlocCPUTopology;
use fluence_libp2p::random_multiaddr::{create_memory_maddr, create_tcp_maddr};
use fluence_libp2p::Transport;
use fs_utils::to_abs_path;
use futures::stream::iter;
use nox::{Connectivity, Node};
use particle_protocol::ProtocolConfig;
use rand::RngCore;
use server_config::{
    persistent_dir, system_services_config, BootstrapConfig, ChainConfig, Network, ResolvedConfig,
    UnresolvedConfig,
};
use tempfile::TempDir;
use test_constants::{EXECUTION_TIMEOUT, IDLE_CONNECTION_TIMEOUT, TRANSPORT_TIMEOUT};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use toy_vms::EasyVM;
use tracing::{Instrument, Span};

const HEALTH_CHECK_POLLING_INTERVAL: Duration = Duration::from_millis(100);

// default bound on the number of computations it can perform simultaneously
const DEFAULT_PARALLELISM: usize = 2;

#[allow(clippy::upper_case_acronyms)]
type AVM = aquamarine::AVMRunner;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct CreatedSwarm {
    pub config: ResolvedConfig,
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
    // tmp dir, must be cleaned
    pub tmp_dir: Arc<TempDir>,
    // management_peer_id
    #[derivative(Debug = "ignore")]
    pub management_keypair: KeyPair,
    // stop signal
    pub exit_outlet: oneshot::Sender<()>,

    pub cancellation_token: CancellationToken,
    // node connectivity
    #[derivative(Debug = "ignore")]
    pub connectivity: Connectivity,
    #[derivative(Debug = "ignore")]
    pub aquamarine_api: AquamarineApi,
    http_listen_addr: SocketAddr,
    pub network_key: NetworkKey,
}

pub async fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, identity).await
}

pub async fn make_swarms_with_cfg<F>(n: usize, mut update_cfg: F) -> Vec<CreatedSwarm>
where
    F: (FnMut(SwarmConfig) -> SwarmConfig),
{
    make_swarms_with(
        n,
        move |bs, maddr| {
            let cfg = update_cfg(SwarmConfig::new(bs, maddr, NetworkKey::random()));
            async move { create_swarm(cfg).await }
        },
        create_memory_maddr,
        identity,
        true,
    )
    .await
}

pub async fn make_swarms_with_transport_and_mocked_vm(
    n: usize,
    transport: Transport,
) -> Vec<CreatedSwarm> {
    let network_key = NetworkKey::random();

    make_swarms_with::<EasyVM, _, _, _, _>(
        n,
        |bs, maddr| async {
            create_swarm_with_runtime(SwarmConfig::new(bs, maddr, network_key.clone()), |_| None)
                .await
        },
        move || match transport {
            Transport::Memory => create_memory_maddr(),
            Transport::Network => create_tcp_maddr(),
        },
        identity,
        true,
    )
    .await
}

pub async fn make_swarms_with_mocked_vm<F, B>(
    n: usize,
    mut update_cfg: F,
    delay: Option<Duration>,
    bootstraps: B,
) -> Vec<CreatedSwarm>
where
    F: (FnMut(SwarmConfig) -> SwarmConfig),
    B: (FnMut(Vec<Multiaddr>) -> Vec<Multiaddr>),
{
    let network_key = NetworkKey::random();

    make_swarms_with::<EasyVM, _, _, _, _>(
        n,
        move |bs, maddr| {
            let cfg = update_cfg(SwarmConfig::new(bs, maddr, network_key.clone()));
            async move { create_swarm_with_runtime(cfg, move |_| delay).await }
        },
        create_memory_maddr,
        bootstraps,
        true,
    )
    .await
}

pub async fn make_swarms_with_keypair(n: usize, host_keypair: KeyPair) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, move |mut cfg| {
        cfg.keypair = host_keypair.clone();
        cfg
    })
    .await
}

type MakeSwarmsData<RT> = (
    PeerId,
    Box<Node<RT>>,
    KeyPair,
    SwarmConfig,
    ResolvedConfig,
    Span,
    PersistenceTask,
);

pub async fn make_swarms_with<RT: AquaRuntime, F, FF, M, B>(
    n: usize,
    mut create_node: F,
    mut create_maddr: M,
    mut bootstraps: B,
    wait_connected: bool,
) -> Vec<CreatedSwarm>
where
    FF: Future<Output = MakeSwarmsData<RT>>,
    F: (FnMut(Vec<Multiaddr>, Multiaddr) -> FF),
    M: (FnMut() -> Multiaddr),
    B: (FnMut(Vec<Multiaddr>) -> Vec<Multiaddr>),
{
    let addrs = (0..n).map(|_| create_maddr()).collect::<Vec<_>>();

    let parallelism = available_parallelism()
        .map(|x| x.get())
        .unwrap_or(DEFAULT_PARALLELISM);

    let nodes: Vec<CreatedSwarm> = stream::iter(addrs.clone())
        .map(|addr| {
            let addrs: Vec<Multiaddr> = addrs.clone().into_iter().filter(|a| a != &addr).collect();
            let bootstraps = bootstraps(addrs);
            let create_node_future = create_node(bootstraps, addr.clone());
            async move {
                let (peer_id, node, management_keypair, input_config, resolved_config, span, task) =
                    create_node_future.await;
                let connectivity = node.connectivity.clone();
                let aquamarine_api = node.aquamarine_api.clone();
                let started_node = node
                    .start(peer_id)
                    .instrument(span)
                    .await
                    .expect("node start");
                let http_listen_addr = started_node
                    .http_listen_addr
                    .expect("could not take http listen addr");
                task.run().await;
                CreatedSwarm {
                    config: resolved_config,
                    peer_id,
                    multiaddr: input_config.listen_on,
                    tmp_dir: input_config.tmp_dir.clone(),
                    management_keypair,
                    exit_outlet: started_node.exit_outlet,
                    cancellation_token: started_node.cancellation_token,
                    connectivity,
                    aquamarine_api,
                    http_listen_addr,
                    network_key: input_config.network_key.clone(),
                }
            }
            .boxed_local()
        })
        .buffer_unordered(parallelism)
        .collect()
        .await;

    // start all nodes
    if wait_connected {
        let addrs = nodes
            .iter()
            .map(|info| info.http_listen_addr)
            .collect::<Vec<_>>();
        wait_connected_on_addrs(addrs).await;
    }

    nodes
}

async fn wait_connected_on_addrs(addrs: Vec<SocketAddr>) {
    let http_client = &reqwest::Client::new();

    let healthcheck = iter(addrs).for_each_concurrent(None, |addr| async move {
        loop {
            let response = http_client
                .get(format!("http://{}/health", addr))
                .send()
                .await
                .unwrap();

            if response.status() == reqwest::StatusCode::OK {
                break;
            }

            tokio::time::sleep(HEALTH_CHECK_POLLING_INTERVAL).await
        }
    });

    healthcheck.await;
}

#[derive(Clone, Debug)]
pub struct NetworkKey([u8; 32]);

impl NetworkKey {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut res: [u8; 32] = Default::default();
        rng.fill_bytes(&mut res);
        NetworkKey(res)
    }
}

impl From<[u8; 32]> for NetworkKey {
    fn from(value: [u8; 32]) -> Self {
        NetworkKey(value)
    }
}

impl From<NetworkKey> for [u8; 32] {
    fn from(value: NetworkKey) -> Self {
        value.0
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct SwarmConfig {
    #[derivative(Debug = "ignore")]
    pub keypair: KeyPair,
    #[derivative(Debug = "ignore")]
    pub management_keypair: KeyPair,
    #[derivative(Debug = "ignore")]
    pub builtins_keypair: KeyPair,
    pub bootstraps: Vec<Multiaddr>,
    pub listen_on: Multiaddr,
    pub transport: Transport,
    pub tmp_dir: Arc<TempDir>,
    pub pool_size: Option<usize>,
    pub builtins_dir: Option<PathBuf>,
    pub spell_base_dir: Option<PathBuf>,
    pub allowed_binaries: Vec<String>,
    pub allowed_effectors: HashMap<String, HashMap<String, String>>,
    pub enabled_system_services: Vec<String>,
    pub extend_system_services: Vec<system_services::PackageDistro>,
    pub override_system_services_config: Option<system_services_config::SystemServicesConfig>,
    pub http_port: u16,
    pub connector_api_endpoint: Option<String>,
    pub chain_config: Option<ChainConfig>,
    pub cc_events_dir: Option<PathBuf>,
    pub network_key: NetworkKey,
}

impl SwarmConfig {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr, network_key: NetworkKey) -> Self {
        let transport = match listen_on.iter().next() {
            Some(Protocol::Memory(_)) => Transport::Memory,
            _ => Transport::Network,
        };
        let tmp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let tmp_dir = Arc::new(tmp_dir);
        Self {
            keypair: KeyPair::generate_ed25519(),
            management_keypair: KeyPair::generate_ed25519(),
            builtins_keypair: KeyPair::generate_ed25519(),
            bootstraps,
            listen_on,
            transport,
            tmp_dir,
            pool_size: <_>::default(),
            builtins_dir: None,
            spell_base_dir: None,
            allowed_binaries: vec!["/usr/bin/ipfs".to_string(), "/usr/bin/curl".to_string()],
            allowed_effectors: HashMap::new(),
            enabled_system_services: vec![],
            extend_system_services: vec![],
            override_system_services_config: None,
            http_port: 0,
            connector_api_endpoint: None,
            chain_config: None,
            cc_events_dir: None,
            network_key,
        }
    }
}

pub struct BaseVmConfig {
    pub peer_id: PeerId,
    pub tmp_dir: PathBuf,
    pub listen_on: Multiaddr,
    pub manager: PeerId,
}

pub fn aqua_vm_config(
    vm_config: BaseVmConfig,
    // startup_peer_id: PeerId,
) -> <AVM as AquaRuntime>::Config {
    let BaseVmConfig {
        peer_id, tmp_dir, ..
    } = vm_config;

    let persistent_dir = persistent_dir(&tmp_dir);
    let air_interpreter = air_interpreter_path(&persistent_dir);
    write_default_air_interpreter(&air_interpreter).expect("write air interpreter");

    VmConfig::new(peer_id, air_interpreter, None, None, None, None, false)
}

pub async fn create_swarm_with_runtime<RT: AquaRuntime>(
    config: SwarmConfig,
    vm_config: impl Fn(BaseVmConfig) -> RT::Config,
) -> MakeSwarmsData<RT> {
    use serde_json::json;

    let format = match &config.keypair {
        KeyPair::Ed25519(_) => "ed25519",
        KeyPair::Rsa(_) => "rsa",
        KeyPair::Secp256k1(_) => "secp256k1",
    };

    let peer_id = libp2p::identity::Keypair::from(config.keypair.clone())
        .public()
        .to_peer_id();
    let parent_span = tracing::info_span!("Node", peer_id = peer_id.to_base58());
    let config_apply_span = tracing::info_span!(parent: &parent_span, "config");
    let node_listen_span = tracing::info_span!(parent: &parent_span, "config");
    let node_creation_span = tracing::info_span!(parent: &parent_span, "config");

    let (node, management_kp, resolved_config, task) = config_apply_span.in_scope(|| {
        let tmp_dir = config.tmp_dir.path().to_path_buf();

        let node_config = json!({
            "network": "dar",
            "base_dir": tmp_dir.to_string_lossy(),
            "root_key_pair": {
                    "format": format,
                    "generate_on_absence": false,
                    "value": base64.encode(config.keypair.to_vec()),
                },
            "builtins_key_pair": {
                    "format": format,
                    "generate_on_absence": false,
                    "value": base64.encode(config.builtins_keypair.to_vec()),
                },
            "builtins_base_dir": config.builtins_dir,
            "external_multiaddresses": [config.listen_on],
            "spell_base_dir": Some(config.spell_base_dir.clone().unwrap_or(to_abs_path(PathBuf::from("spell")))),
            "http_port": config.http_port,
            "listen_ip": "127.0.0.1",
            "cc_events_dir": config.cc_events_dir,
        });

        let node_config: UnresolvedConfig =
            UnresolvedConfig::deserialize(node_config).expect("created_swarm: deserialize config");

        let mut resolved = node_config.resolve().expect("failed to resolve config");
        resolved.node_config.transport_config.transport = Transport::Memory;
        resolved.node_config.transport_config.socket_timeout = TRANSPORT_TIMEOUT;
        resolved.node_config.protocol_config =
            ProtocolConfig::new(TRANSPORT_TIMEOUT, TRANSPORT_TIMEOUT);
        resolved.network=Network::Custom(config.network_key.clone().into());

        resolved.node_config.bootstrap_nodes = config.bootstraps.clone();
        resolved.node_config.bootstrap_config = BootstrapConfig::zero();
        resolved.node_config.bootstrap_frequency = 1;

        resolved.metrics_config.metrics_enabled = false;
        resolved.node_config.health_config.health_check_enabled = true;

        resolved.node_config.allow_local_addresses = true;

        resolved.node_config.aquavm_pool_size = config.pool_size.unwrap_or(1);
        resolved.node_config.particle_execution_timeout = EXECUTION_TIMEOUT;
        resolved.node_config.transport_config.connection_idle_timeout = IDLE_CONNECTION_TIMEOUT;

        let allowed_effectors = config.allowed_effectors.iter().map(|(cid, binaries)| {
            (Hash::from_string(cid).unwrap(), binaries.clone())
        }).collect::<_>();
        resolved.node_config.allowed_effectors = allowed_effectors;

        if let Some(config) = config.override_system_services_config.clone() {
            resolved.system_services = config;
        }

        // `enable_system_services` has higher priority then `enable` field of the SystemServicesConfig
        resolved.system_services.enable = config
            .enabled_system_services
            .iter()
            .map(|service| {
                system_services_config::ServiceKey::from_string(service)
                    .unwrap_or_else(|| panic!("service {service} doesn't exist"))
            })
            .collect();

        if let Some(endpoint) = config.connector_api_endpoint.clone() {
            resolved.system_services.decider.network_api_endpoint = endpoint;
        }

        let management_peer_id = libp2p::identity::Keypair::from(config.management_keypair.clone())
            .public()
            .to_peer_id();
        resolved.node_config.management_peer_id = management_peer_id;
        resolved.chain_config = config.chain_config.clone();

        let vm_config = vm_config(BaseVmConfig {
            peer_id,
            tmp_dir: tmp_dir.clone(),
            listen_on: config.listen_on.clone(),
            manager: management_peer_id,
        });

        let data_store_config = DataStoreConfig::new(tmp_dir.clone());

        let system_services_config = resolved.system_services.clone();
        let system_service_distros =
            system_services::SystemServiceDistros::default_from(system_services_config)
                .expect("Failed to get default system service distros")
                .extend(config.extend_system_services.clone());

        let cpu_topology = HwlocCPUTopology::new().expect("Failed to get cpu topology");

        let (core_distributor, task) = PersistentCoreDistributor::from_path(
            tmp_dir.join("test-core-state.toml"),
            1,
            CoreRange::default(),
            AcquireStrategy::RoundRobin,
            &cpu_topology
        ).expect("Failed to create core distributor");

        let thread_pinner = Arc::new(ccp_test_utils::pinning::DUMMY);

        let node = Node::new(
            resolved.clone(),
            core_distributor,
            thread_pinner,
            vm_config,
            data_store_config,
            "some version",
            "some version",
            system_service_distros,
        );
        (node, config.management_keypair.clone(), resolved, task)
    });

    let mut node = node
        .instrument(node_creation_span)
        .await
        .expect("create node");

    node_listen_span.in_scope(|| {
        node.listen(vec![config.listen_on.clone()]).expect("listen");
        (
            node.scope.get_host_peer_id(),
            node,
            management_kp,
            config,
            resolved_config,
            parent_span.clone(),
            task,
        )
    })
}

pub async fn create_swarm(config: SwarmConfig) -> MakeSwarmsData<AVMRunner> {
    create_swarm_with_runtime(config, aqua_vm_config).await
}
