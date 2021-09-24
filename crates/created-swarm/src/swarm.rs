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
use std::path::Path;
use std::{path::PathBuf, time::Duration};

use async_std::task;
use derivative::Derivative;
use fluence_identity::KeyPair;
use futures::channel::mpsc::unbounded;
use futures::{stream::iter, StreamExt};
use libp2p::core::multiaddr::Protocol;
use libp2p::{core::Multiaddr, identity::Keypair, PeerId};
use serde::Deserialize;
use trust_graph::{Certificate, InMemoryStorage, TrustGraph};

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::VmPoolConfig;
use aquamarine::{AquaRuntime, VmConfig, AVM};
use builtins_deployer::BuiltinsDeployer;
use config_utils::to_peer_id;
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use fluence_libp2p::random_multiaddr::{create_memory_maddr, create_tcp_maddr};
use fluence_libp2p::types::OneshotOutlet;
use fluence_libp2p::{build_memory_transport, build_transport, RandomPeerId, Transport};
use fs_utils::{make_tmp_dir_peer_id, to_abs_path};
use particle_node::{Connectivity, Node};
use particle_protocol::ProtocolConfig;
use script_storage::ScriptStorageConfig;
use script_storage::{ScriptStorageApi, ScriptStorageBackend};
use server_config::{
    BootstrapConfig, NetworkConfig, NodeConfig, ResolvedConfig, ServicesConfig, UnresolvedConfig,
};
use test_constants::{EXECUTION_TIMEOUT, KEEP_ALIVE_TIMEOUT, PARTICLE_TTL, TRANSPORT_TIMEOUT};
use toy_vms::EasyVM;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct CreatedSwarm {
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
    // tmp dir, must be cleaned
    pub tmp_dir: PathBuf,
    // management_peer_id
    #[derivative(Debug = "ignore")]
    pub management_keypair: KeyPair,
    // stop signal
    pub outlet: OneshotOutlet<()>,
    // node connectivity
    pub connectivity: Connectivity,
}

pub fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, identity)
}

pub fn make_swarms_with_cfg<F>(n: usize, mut update_cfg: F) -> Vec<CreatedSwarm>
where
    F: FnMut(SwarmConfig) -> SwarmConfig,
{
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(update_cfg(SwarmConfig::new(bs, maddr))),
        create_memory_maddr,
        identity,
        true,
    )
}

pub fn make_swarms_with_transport_and_mocked_vm(
    n: usize,
    transport: Transport,
) -> Vec<CreatedSwarm> {
    make_swarms_with::<EasyVM, _, _, _>(
        n,
        |bs, maddr| create_swarm_with_runtime(SwarmConfig::new(bs, maddr), |_, _, _, _| None),
        || match transport {
            Transport::Memory => create_memory_maddr(),
            Transport::Network => create_tcp_maddr(),
        },
        identity,
        true,
    )
}

pub fn make_swarms_with_mocked_vm<F, B>(
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
            create_swarm_with_runtime(update_cfg(SwarmConfig::new(bs, maddr)), |_, _, _, _| delay)
        },
        create_memory_maddr,
        bootstraps,
        true,
    )
}

pub fn make_swarms_with_keypair(n: usize, keypair: KeyPair) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        cfg.keypair = keypair.clone();
        cfg
    })
}

pub fn make_swarms_with_builtins(
    n: usize,
    path: &Path,
    keypair: Option<KeyPair>,
) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        if let Some(keypair) = &keypair {
            cfg.keypair = keypair.clone();
        }
        cfg.builtins_dir = Some(to_abs_path(path.into()));
        cfg
    })
}

pub fn make_swarms_with<RT: AquaRuntime, F, M, B>(
    n: usize,
    mut create_node: F,
    mut create_maddr: M,
    mut bootstraps: B,
    wait_connected: bool,
) -> Vec<CreatedSwarm>
where
    F: FnMut(Vec<Multiaddr>, Multiaddr) -> (PeerId, Box<Node<RT>>, KeyPair, SwarmConfig),
    M: FnMut() -> Multiaddr,
    B: FnMut(Vec<Multiaddr>) -> Vec<Multiaddr>,
{
    let addrs = (0..n).map(|_| create_maddr()).collect::<Vec<_>>();
    let nodes = addrs
        .iter()
        .map(|addr| {
            #[rustfmt::skip]
            let addrs = addrs.iter().filter(|&a| a != addr).cloned().collect::<Vec<_>>();
            let bootstraps = bootstraps(addrs);
            let bootstraps_num = bootstraps.len();
            let (id, node, m_kp, config) = create_node(bootstraps, addr.clone());
            ((id, m_kp, config), node, bootstraps_num)
        })
        .collect::<Vec<_>>();

    let pools = iter(
        nodes
            .iter()
            .map(|(_, n, bootstraps_num)| (n.connectivity.clone(), *bootstraps_num))
            .collect::<Vec<_>>(),
    );
    let connected = pools.for_each_concurrent(None, |(pool, bootstraps_num)| async move {
        let pool = AsRef::<ConnectionPoolApi>::as_ref(&pool);
        let mut events = pool.lifecycle_events();
        loop {
            let num = pool.count_connections().await;
            if num >= bootstraps_num {
                break;
            }
            // wait until something changes
            events.next().await;
        }
    });

    // start all nodes
    let infos = nodes
        .into_iter()
        .map(|((peer_id, management_keypair, config), node, _)| {
            let connectivity = node.connectivity.clone();
            let startup_peer_id = node.builtins_management_peer_id;
            let local_peer_id = node.local_peer_id;
            let outlet = node.start().expect("node start");

            CreatedSwarm {
                peer_id,
                multiaddr: config.listen_on,
                tmp_dir: config.tmp_dir.unwrap(),
                management_keypair,
                outlet,
                connectivity,
            }
        })
        .collect();

    if wait_connected {
        task::block_on(connected);
    }

    infos
}

#[derive(Default, Clone, Debug)]
pub struct Trust {
    pub root_weights: Vec<(fluence_identity::PublicKey, u32)>,
    pub certificates: Vec<Certificate>,
    pub cur_time: Duration,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct SwarmConfig {
    #[derivative(Debug = "ignore")]
    pub keypair: fluence_identity::KeyPair,
    pub bootstraps: Vec<Multiaddr>,
    pub listen_on: Multiaddr,
    pub trust: Option<Trust>,
    pub transport: Transport,
    pub tmp_dir: Option<PathBuf>,
    pub pool_size: Option<usize>,
    pub builtins_dir: Option<PathBuf>,
}

impl SwarmConfig {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        let transport = match listen_on.iter().next() {
            Some(Protocol::Memory(_)) => Transport::Memory,
            _ => Transport::Network,
        };
        Self {
            keypair: fluence_identity::KeyPair::generate_ed25519(),
            bootstraps,
            listen_on,
            transport,
            trust: <_>::default(),
            tmp_dir: None,
            pool_size: <_>::default(),
            builtins_dir: None,
        }
    }

    pub fn with_trust(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr, trust: Trust) -> Self {
        let mut this = Self::new(bootstraps, listen_on);
        this.trust = Some(trust);
        this
    }
}

pub struct BaseVmConfig {
    pub peer_id: PeerId,
    pub tmp_dir: PathBuf,
    pub listen_on: Multiaddr,
    pub manager: PeerId,
}

pub fn aqua_vm_config(
    connectivity: Connectivity,
    script_storage_api: ScriptStorageApi,
    vm_config: BaseVmConfig,
    startup_peer_id: PeerId,
) -> <AVM as AquaRuntime>::Config {
    let BaseVmConfig {
        peer_id,
        tmp_dir,
        listen_on,
        manager,
    } = vm_config;

    let air_interpreter = air_interpreter_path(&tmp_dir);
    write_default_air_interpreter(&air_interpreter).expect("write air interpreter");

    let avm_base_dir = tmp_dir.join("interpreter");
    let vm_config = VmConfig::new(peer_id, avm_base_dir.clone(), air_interpreter);

    let services_config = ServicesConfig::new(
        peer_id,
        config_utils::services_dir(&tmp_dir),
        config_utils::particles_vault_dir(&avm_base_dir),
        <_>::default(),
        manager,
        startup_peer_id,
    )
    .expect("create services config");

    vm_config
}

pub fn create_swarm_with_runtime<RT: AquaRuntime>(
    mut config: SwarmConfig,
    vm_config: impl Fn(Connectivity, ScriptStorageApi, BaseVmConfig, PeerId) -> RT::Config,
) -> (PeerId, Box<Node<RT>>, KeyPair, SwarmConfig) {
    use serde_json::json;

    let cfg = config.clone();

    let format = match &config.keypair {
        KeyPair::Ed25519(_) => "ed25519",
        KeyPair::Rsa(_) => "rsa",
        KeyPair::Secp256k1(_) => "secp256k1",
    };

    let peer_id = libp2p::identity::Keypair::from(config.keypair.clone())
        .public()
        .into_peer_id();

    if config.tmp_dir.is_none() {
        config.tmp_dir = Some(make_tmp_dir_peer_id(peer_id.to_string()));
    }
    let tmp_dir = config.tmp_dir.as_ref().unwrap();
    std::fs::create_dir_all(tmp_dir).expect("create tmp dir");

    let node_config = json!({
        "base_dir": tmp_dir.to_string_lossy(),
        "root_key_pair": {
            "format": format,
            "generate_on_absence": "false",
            "value": bs58::encode(config.keypair.to_vec()).into_string(),
        }
    });

    let node_config: UnresolvedConfig =
        UnresolvedConfig::deserialize(node_config).expect("empty config");

    let mut resolved = node_config.resolve();
    resolved.node_config.transport_config.transport = Transport::Memory;
    resolved.node_config.transport_config.socket_timeout = TRANSPORT_TIMEOUT;
    resolved.node_config.protocol_config =
        ProtocolConfig::new(TRANSPORT_TIMEOUT, KEEP_ALIVE_TIMEOUT, TRANSPORT_TIMEOUT);

    resolved.node_config.bootstrap_nodes = config.bootstraps;
    resolved.node_config.bootstrap_config = BootstrapConfig::zero();
    resolved.node_config.bootstrap_frequency = 1;

    resolved.prometheus_config.prometheus_enabled = false;

    resolved.node_config.allow_local_addresses = true;
    resolved.node_config.particle_processing_timeout = Duration::from_secs(45);
    // resolved.node_config.listen_config.listen_multiaddrs = vec![.clone()];

    resolved.node_config.aquavm_pool_size = config.pool_size.unwrap_or(1);
    resolved.node_config.particle_execution_timeout = EXECUTION_TIMEOUT;

    let management_kp = fluence_identity::KeyPair::generate_ed25519();
    let management_peer_id = libp2p::identity::Keypair::from(management_kp.clone())
        .public()
        .into_peer_id();
    resolved.node_config.management_peer_id = management_peer_id;

    let mut node = Node::new(resolved, "some version").expect("create node");
    node.listen(vec![config.listen_on]).expect("listen");

    (node.local_peer_id, node, management_kp, cfg)

    // TODO: this requires ability to convert public keys to PeerId
    // if let Some(trust) = config.trust {
    //     node_config.root_weights = trust.root_weights.into_iter().map(|(pk, weight)| {
    //         pk.
    //     }).collect();
    // }
}

// pub fn create_swarm_with_runtime<RT: AquaRuntime>(
//     mut config: SwarmConfig,
//     vm_config: impl Fn(Connectivity, ScriptStorageApi, BaseVmConfig, PeerId) -> RT::Config,
// ) -> (PeerId, Box<Node<RT>>, Keypair, SwarmConfig) {
//     #[rustfmt::skip]
//     let SwarmConfig { bootstraps, listen_on, trust, transport, .. } = config.clone();
//
//     let peer_id = to_peer_id(&config.keypair);
//
//     if config.tmp_dir.is_none() {
//         config.tmp_dir = Some(make_tmp_dir_peer_id(peer_id.to_string()));
//     }
//
//     let tmp_dir = config.tmp_dir.as_ref().unwrap();
//     let management_kp = Keypair::generate_ed25519();
//     let m_public_key = management_kp.public();
//     let m_id = PeerId::from(m_public_key);
//
//     let root_weights: &[_] = trust.as_ref().map_or(&[], |t| &t.root_weights);
//     let mut trust_graph = {
//         let storage = InMemoryStorage::new_in_memory(root_weights.to_vec());
//         TrustGraph::new(storage)
//     };
//     if let Some(trust) = trust {
//         for cert in trust.certificates.into_iter() {
//             trust_graph.add(cert, trust.cur_time).expect("add cert");
//         }
//     }
//
//     let protocol_config =
//         ProtocolConfig::new(TRANSPORT_TIMEOUT, KEEP_ALIVE_TIMEOUT, TRANSPORT_TIMEOUT);
//
//     let network_config = NetworkConfig {
//         key_pair: config.keypair.clone(),
//         local_peer_id: peer_id.clone(),
//         node_version: "<node version>",
//         trust_graph,
//         bootstrap_nodes: bootstraps.clone(),
//         bootstrap: BootstrapConfig::zero(),
//         registry: None,
//         protocol_config,
//         kademlia_config: Default::default(),
//         particle_queue_buffer: 100,
//         // particle_parallelism: Some(16),
//         bootstrap_frequency: 1,
//         allow_local_addresses: true,
//         // particle_timeout: Duration::from_secs(45),
//     };
//
//     let transport = match transport {
//         Transport::Memory => build_memory_transport(config.keypair.clone(), TRANSPORT_TIMEOUT),
//         Transport::Network => build_network_transport(config.keypair.clone(), TRANSPORT_TIMEOUT),
//     };
//
//     let (swarm, network_api, particle_stream) =
//         Node::swarm(peer_id, network_config, transport, vec![listen_on.clone()]);
//
//     let connectivity = network_api.connectivity();
//     let (particle_failures_out, particle_failures_in) = unbounded();
//     let (script_storage_api, script_storage_backend) = {
//         let script_storage_config = ScriptStorageConfig {
//             timer_resolution: Duration::from_millis(500),
//             max_failures: 1,
//             particle_ttl: Duration::from_secs(5),
//             peer_id,
//         };
//
//         let pool: &ConnectionPoolApi = connectivity.as_ref();
//         ScriptStorageBackend::new(pool.clone(), particle_failures_in, script_storage_config)
//     };
//
//     let pool_size = config.pool_size.unwrap_or(1);
//     let pool_config = VmPoolConfig::new(pool_size, EXECUTION_TIMEOUT);
//
//     std::fs::create_dir_all(tmp_dir).expect("create tmp dir");
//
//     let builtins_management_peer_id = RandomPeerId::random();
//     let vm_config = vm_config(
//         connectivity,
//         script_storage_api,
//         BaseVmConfig {
//             peer_id,
//             tmp_dir: tmp_dir.clone(),
//             listen_on: listen_on.clone(),
//             manager: m_id,
//         },
//         builtins_management_peer_id,
//     );
//
//     let mut node = Node::with(
//         particle_stream,
//         peer_id,
//         swarm,
//         network_api,
//         script_storage_backend,
//         vm_config,
//         pool_config,
//         particle_failures_out,
//         None,
//         "0.0.0.0:0".parse().unwrap(),
//         builtins_management_peer_id,
//         bootstraps,
//     );
//
//     node.listen(vec![listen_on]).expect("listen");
//
//     (peer_id, node, management_kp, config)
// }

pub fn create_swarm(config: SwarmConfig) -> (PeerId, Box<Node<AVM>>, KeyPair, SwarmConfig) {
    create_swarm_with_runtime(config, aqua_vm_config)
}
