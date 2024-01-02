/*
 * Copyright 2021 Fluence Labs Limited
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

use std::convert::identity;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{path::PathBuf, time::Duration};

use derivative::Derivative;
use fluence_keypair::KeyPair;
use futures::{FutureExt, StreamExt};
use libp2p::core::multiaddr::Protocol;
use libp2p::{core::Multiaddr, PeerId};
use serde::Deserialize;

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::{AVMRunner, AquamarineApi};
use aquamarine::{AquaRuntime, DataStoreConfig, VmConfig};
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use fluence_libp2p::random_multiaddr::{create_memory_maddr, create_tcp_maddr};
use fluence_libp2p::Transport;
use fs_utils::to_abs_path;
use futures::future::{join_all, BoxFuture};
use futures::stream::iter;
use nox::{Connectivity, Node};
use particle_protocol::ProtocolConfig;
use server_config::{system_services_config, BootstrapConfig, UnresolvedConfig};
use tempfile::TempDir;
use test_constants::{EXECUTION_TIMEOUT, TRANSPORT_TIMEOUT};
use tokio::sync::oneshot;
use toy_vms::EasyVM;
use tracing::{Instrument, Span};

const HEALTH_CHECK_POLLING_INTERVAL: Duration = Duration::from_millis(100);

#[allow(clippy::upper_case_acronyms)]
type AVM = aquamarine::AVMRunner;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct CreatedSwarm {
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
    // tmp dir, must be cleaned
    pub tmp_dir: Arc<TempDir>,
    // management_peer_id
    #[derivative(Debug = "ignore")]
    pub management_keypair: KeyPair,
    // stop signal
    pub exit_outlet: oneshot::Sender<()>,
    // node connectivity
    #[derivative(Debug = "ignore")]
    pub connectivity: Connectivity,
    #[derivative(Debug = "ignore")]
    pub aquamarine_api: AquamarineApi,
    http_listen_addr: SocketAddr,
}

pub async fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, identity).await
}

pub async fn make_swarms_with_cfg<F>(n: usize, mut update_cfg: F) -> Vec<CreatedSwarm>
where
    F: FnMut(SwarmConfig) -> SwarmConfig,
{
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(update_cfg(SwarmConfig::new(bs, maddr))).boxed(),
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
    make_swarms_with::<EasyVM, _, _, _>(
        n,
        |bs, maddr| create_swarm_with_runtime(SwarmConfig::new(bs, maddr), |_| None).boxed(),
        || match transport {
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
    F: FnMut(SwarmConfig) -> SwarmConfig,
    B: FnMut(Vec<Multiaddr>) -> Vec<Multiaddr>,
{
    make_swarms_with::<EasyVM, _, _, _>(
        n,
        |bs, maddr| {
            create_swarm_with_runtime(update_cfg(SwarmConfig::new(bs, maddr)), move |_| delay)
                .boxed()
        },
        create_memory_maddr,
        bootstraps,
        true,
    )
    .await
}

pub async fn make_swarms_with_keypair(
    n: usize,
    keypair: KeyPair,
    spell_base_dir: Option<String>,
) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        cfg.keypair = keypair.clone();
        cfg.spell_base_dir = spell_base_dir.clone().map(PathBuf::from);
        cfg
    })
    .await
}

pub async fn make_swarms_with<RT: AquaRuntime, F, M, B>(
    n: usize,
    mut create_node: F,
    mut create_maddr: M,
    mut bootstraps: B,
    wait_connected: bool,
) -> Vec<CreatedSwarm>
where
    F: FnMut(
        Vec<Multiaddr>,
        Multiaddr,
    ) -> BoxFuture<'static, (PeerId, Box<Node<RT>>, KeyPair, SwarmConfig, Span)>,
    M: FnMut() -> Multiaddr,
    B: FnMut(Vec<Multiaddr>) -> Vec<Multiaddr>,
{
    let addrs = (0..n).map(|_| create_maddr()).collect::<Vec<_>>();
    let nodes = addrs
        .iter()
        .map(|addr| {
            let addrs = addrs
                .iter()
                .filter(|&a| a != addr)
                .cloned()
                .collect::<Vec<_>>();
            let bootstraps = bootstraps(addrs);
            create_node(bootstraps, addr.clone())
        })
        .collect::<Vec<_>>();

    // start all nodes
    let infos = join_all(nodes.into_iter().map(move |tasks| {
        async move {
            let (peer_id, node, management_keypair, config, span) = tasks.await;
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
            CreatedSwarm {
                peer_id,
                multiaddr: config.listen_on,
                tmp_dir: config.tmp_dir.clone(),
                management_keypair,
                exit_outlet: started_node.exit_outlet,
                connectivity,
                aquamarine_api,
                http_listen_addr,
            }
        }
        .boxed()
    }))
    .await;

    if wait_connected {
        let addrs = infos
            .iter()
            .map(|info| info.http_listen_addr)
            .collect::<Vec<_>>();
        wait_connected_on_addrs(addrs).await;
    }

    infos
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

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct SwarmConfig {
    #[derivative(Debug = "ignore")]
    pub keypair: fluence_keypair::KeyPair,
    #[derivative(Debug = "ignore")]
    pub builtins_keypair: fluence_keypair::KeyPair,
    pub bootstraps: Vec<Multiaddr>,
    pub listen_on: Multiaddr,
    pub transport: Transport,
    pub tmp_dir: Arc<TempDir>,
    pub pool_size: Option<usize>,
    pub builtins_dir: Option<PathBuf>,
    pub spell_base_dir: Option<PathBuf>,
    pub allowed_binaries: Vec<String>,
    pub enabled_system_services: Vec<String>,
    pub extend_system_services: Vec<system_services::PackageDistro>,
    pub override_system_services_config: Option<system_services_config::SystemServicesConfig>,
    pub http_port: u16,
    pub connector_api_endpoint: Option<String>,
}

impl SwarmConfig {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        let transport = match listen_on.iter().next() {
            Some(Protocol::Memory(_)) => Transport::Memory,
            _ => Transport::Network,
        };
        let tmp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let tmp_dir = Arc::new(tmp_dir);
        Self {
            keypair: fluence_keypair::KeyPair::generate_ed25519(),
            builtins_keypair: fluence_keypair::KeyPair::generate_ed25519(),
            bootstraps,
            listen_on,
            transport,
            tmp_dir,
            pool_size: <_>::default(),
            builtins_dir: None,
            spell_base_dir: None,
            allowed_binaries: vec!["/usr/bin/ipfs".to_string(), "/usr/bin/curl".to_string()],
            enabled_system_services: vec![],
            extend_system_services: vec![],
            override_system_services_config: None,
            http_port: 0,
            connector_api_endpoint: None,
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

    let air_interpreter = air_interpreter_path(&tmp_dir);
    write_default_air_interpreter(&air_interpreter).expect("write air interpreter");

    VmConfig::new(peer_id, air_interpreter, None)
}

pub async fn create_swarm_with_runtime<RT: AquaRuntime>(
    config: SwarmConfig,
    vm_config: impl Fn(BaseVmConfig) -> RT::Config,
) -> (PeerId, Box<Node<RT>>, KeyPair, SwarmConfig, Span) {
    use serde_json::json;

    let format = match &config.keypair {
        KeyPair::Ed25519(_) => "ed25519",
        KeyPair::Rsa(_) => "rsa",
        KeyPair::Secp256k1(_) => "secp256k1",
    };

    let peer_id = libp2p::identity::Keypair::from(config.keypair.clone())
        .public()
        .to_peer_id();
    let spawn = tracing::info_span!("Node", peer_id = peer_id.to_base58());

    let tmp_dir = config.tmp_dir.path().to_path_buf();
    let _enter = spawn.enter();

    let node_config = json!({
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
        "listen_ip": "127.0.0.1"
    });

    let node_config: UnresolvedConfig =
        UnresolvedConfig::deserialize(node_config).expect("created_swarm: deserialize config");

    let mut resolved = node_config.resolve().expect("failed to resolve config");

    resolved.node_config.transport_config.transport = Transport::Memory;
    resolved.node_config.transport_config.socket_timeout = TRANSPORT_TIMEOUT;
    resolved.node_config.protocol_config =
        ProtocolConfig::new(TRANSPORT_TIMEOUT, TRANSPORT_TIMEOUT);

    resolved.node_config.bootstrap_nodes = config.bootstraps.clone();
    resolved.node_config.bootstrap_config = BootstrapConfig::zero();
    resolved.node_config.bootstrap_frequency = 1;

    resolved.metrics_config.metrics_enabled = false;

    resolved.node_config.allow_local_addresses = true;

    resolved.node_config.aquavm_pool_size = config.pool_size.unwrap_or(1);
    resolved.node_config.particle_execution_timeout = EXECUTION_TIMEOUT;

    resolved.node_config.allowed_binaries = config.allowed_binaries.clone();

    if let Some(config) = config.override_system_services_config.clone() {
        resolved.system_services = config;
    }
    // `enable_system_services` has higher priority then `enable` field of the SystemServicesConfig
    resolved.system_services.enable = config
        .enabled_system_services
        .iter()
        .map(|service| {
            system_services_config::ServiceKey::from_string(service)
                .expect(&format!("service {service} doesn't exist"))
        })
        .collect();

    if let Some(endpoint) = config.connector_api_endpoint.clone() {
        resolved.system_services.decider.network_api_endpoint = endpoint;
    }

    let management_kp = fluence_keypair::KeyPair::generate_ed25519();
    let management_peer_id = libp2p::identity::Keypair::from(management_kp.clone())
        .public()
        .to_peer_id();
    resolved.node_config.management_peer_id = management_peer_id;

    let vm_config = vm_config(BaseVmConfig {
        peer_id,
        tmp_dir: tmp_dir.clone(),
        listen_on: config.listen_on.clone(),
        manager: management_peer_id,
    });

    let datas_tore_config = DataStoreConfig::new(tmp_dir.clone());

    let system_services_config = resolved.system_services.clone();
    let system_service_distros =
        system_services::SystemServiceDistros::default_from(system_services_config)
            .expect("Failed to get default system service distros")
            .extend(config.extend_system_services.clone());

    let mut node = Node::new(
        resolved,
        vm_config,
        datas_tore_config,
        "some version",
        "some version",
        system_service_distros,
    )
    .await
    .expect("create node");
    node.listen(vec![config.listen_on.clone()]).expect("listen");

    (
        node.scope.get_host_peer_id(),
        node,
        management_kp,
        config,
        spawn.clone(),
    )
}

pub async fn create_swarm(
    config: SwarmConfig,
) -> (PeerId, Box<Node<AVMRunner>>, KeyPair, SwarmConfig, Span) {
    create_swarm_with_runtime(config, aqua_vm_config).await
}
