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

// reexport
pub use fluence_client::Transport;

use crate::EasyVM;

use aquamarine::VmPoolConfig;
use aquamarine::{AquaRuntime, VmConfig, AVM};
use config_utils::{modules_dir, to_peer_id};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use fluence_libp2p::types::OneshotOutlet;
use fluence_libp2p::{build_memory_transport, build_transport};
use particle_node::{BuiltinsDeployer, Connectivity, Node};
use particle_protocol::ProtocolConfig;
use script_storage::ScriptStorageConfig;
use script_storage::{ScriptStorageApi, ScriptStorageBackend};
use server_config::{BootstrapConfig, NetworkConfig, ServicesConfig};
use trust_graph::{Certificate, InMemoryStorage, TrustGraph};

use async_std::task;
use derivative::Derivative;
use eyre::WrapErr;
use fs_utils::make_tmp_dir_peer_id;
use futures::channel::mpsc::unbounded;
use futures::{stream::iter, StreamExt};
use libp2p::core::multiaddr::Protocol;
use libp2p::{core::Multiaddr, identity::Keypair, PeerId};
use rand::Rng;
use serde_json::{json, Value as JValue};
use services_utils::put_aquamarine;
use std::collections::HashMap;
use std::convert::identity;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{path::PathBuf, time::Duration};

/// Utility functions for tests.

pub type Result<T> = eyre::Result<T>;

/// In debug, VM startup time is big, account for that
#[cfg(debug_assertions)]
pub static TIMEOUT: Duration = Duration::from_secs(150);
#[cfg(not(debug_assertions))]
pub static TIMEOUT: Duration = Duration::from_secs(15);

pub static SHORT_TIMEOUT: Duration = Duration::from_millis(300);
pub static KAD_TIMEOUT: Duration = Duration::from_millis(500);
pub static TRANSPORT_TIMEOUT: Duration = Duration::from_millis(500);
pub static KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(10);
pub static EXECUTION_TIMEOUT: Duration = Duration::from_millis(5000);
pub static PARTICLE_TTL: u32 = 20000;

pub fn get_cert() -> Certificate {
    use std::str::FromStr;

    Certificate::from_str(
        r#"11
1111
EqpwyPYjbRbGPcp7Q1UtSnkeCDG9x3JrY96strN4uaXv
4Td1uTWzqWp1PyUzoUZyvWNjgPWQKpMFDYeqzoAJSXHQtkVispifSrnnqBFM8yFPkgmSHwQ4kTuACBifjoRryvFK
18446744073709551615
1589892496362
DYVjCCtVPnJNEDfRDzYn6a2GKJ6Qn4FNVwDhEAQBvdQS
3Tt8UxBr2pixgMMbRM4gnJDkX3zH3NnS5q4A5fCj3taMLpS2QathgUqkW4KHysQLeRoGxy3JNVtYEWLsL6kySrqv
1621450096362
1589892496362
HFF3V9XXbhdTLWGVZkJYd9a7NyuD5BLWLdwc4EFBcCZa
38FUPbDMrrb1FaRoRTsupjqysaH3vvpJJgp9NxLFBjBYoU353bb6LkDZLDsNwvnpVysrs6TdHeZAAe3iXrJuGLkn
101589892496363
1589892496363
"#,
    )
    .expect("deserialize cert")
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct CreatedSwarm {
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
    // tmp dir, must be cleaned
    pub tmp_dir: PathBuf,
    // management_peer_id
    #[derivative(Debug = "ignore")]
    pub management_keypair: Keypair,
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

pub fn make_swarms_with_keypair(n: usize, keypair: Keypair) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        cfg.keypair = keypair.clone();
        cfg
    })
}

pub fn make_swarms_with_builtins(
    n: usize,
    path: &Path,
    keypair: Option<Keypair>,
) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        if let Some(keypair) = &keypair {
            cfg.keypair = keypair.clone();
        }
        cfg.builtins_dir = Some(path.into());
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
    F: FnMut(Vec<Multiaddr>, Multiaddr) -> (PeerId, Box<Node<RT>>, Keypair, SwarmConfig),
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
            .map(|(_, n, bootstraps_num)| (n.network_api.connectivity(), *bootstraps_num))
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
            let connectivity = node.network_api.connectivity();
            let stepper = node.stepper_pool_api.clone();
            let startup_peer_id = to_peer_id(&node.startup_keypair);
            let local_peer_id = node.local_peer_id;
            let outlet = node.start();

            if let Some(builtins_dir) = config.builtins_dir {
                let mut builtin_loader =
                    BuiltinsDeployer::new(startup_peer_id, local_peer_id, stepper, builtins_dir);

                builtin_loader
                    .deploy_builtin_services()
                    .expect("builtins deployed");
            }

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
    pub keypair: Keypair,
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
            keypair: Keypair::generate_ed25519(),
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

    let avm_base_dir = tmp_dir.join("stepper");
    let air_interpreter = put_aquamarine(modules_dir(&avm_base_dir));

    let vm_config =
        VmConfig::new(peer_id, avm_base_dir, air_interpreter).expect("create vm config");

    let services_config = ServicesConfig::new(
        peer_id,
        tmp_dir.join("services"),
        <_>::default(),
        manager,
        startup_peer_id,
    )
    .expect("create services config");

    let host_closures = Node::host_closures(
        connectivity,
        vec![listen_on],
        services_config,
        script_storage_api,
    );

    (vm_config, host_closures.descriptor())
}

pub fn create_swarm_with_runtime<RT: AquaRuntime>(
    mut config: SwarmConfig,
    vm_config: impl Fn(Connectivity, ScriptStorageApi, BaseVmConfig, PeerId) -> RT::Config,
) -> (PeerId, Box<Node<RT>>, Keypair, SwarmConfig) {
    #[rustfmt::skip]
    let SwarmConfig { bootstraps, listen_on, trust, transport, .. } = config.clone();

    let peer_id = to_peer_id(&config.keypair);

    if config.tmp_dir.is_none() {
        config.tmp_dir = Some(make_tmp_dir_peer_id(peer_id.to_string()));
    }

    let tmp_dir = config.tmp_dir.as_ref().unwrap();
    let management_kp = Keypair::generate_ed25519();
    let m_public_key = management_kp.public();
    let m_id = PeerId::from(m_public_key);

    let root_weights: &[_] = trust.as_ref().map_or(&[], |t| &t.root_weights);
    let mut trust_graph = {
        let storage = InMemoryStorage::new_in_memory(root_weights.to_vec());
        TrustGraph::new(storage)
    };
    if let Some(trust) = trust {
        for cert in trust.certificates.into_iter() {
            trust_graph.add(cert, trust.cur_time).expect("add cert");
        }
    }

    let protocol_config =
        ProtocolConfig::new(TRANSPORT_TIMEOUT, KEEP_ALIVE_TIMEOUT, TRANSPORT_TIMEOUT);

    let network_config = NetworkConfig {
        key_pair: config.keypair.clone(),
        local_peer_id: peer_id.clone(),
        trust_graph,
        bootstrap_nodes: bootstraps.clone(),
        bootstrap: BootstrapConfig::zero(),
        registry: None,
        protocol_config,
        kademlia_config: Default::default(),
        particle_queue_buffer: 100,
        particle_parallelism: 16,
        bootstrap_frequency: 1,
        allow_local_addresses: true,
        particle_timeout: Duration::from_secs(45),
    };

    let transport = match transport {
        Transport::Memory => build_memory_transport(config.keypair.clone(), TRANSPORT_TIMEOUT),
        Transport::Network => build_transport(config.keypair.clone(), TRANSPORT_TIMEOUT),
    };

    let (swarm, network_api) =
        Node::swarm(peer_id, network_config, transport, vec![listen_on.clone()]);

    let connectivity = network_api.connectivity();
    let (particle_failures_out, particle_failures_in) = unbounded();
    let (script_storage_api, script_storage_backend) = {
        let script_storage_config = ScriptStorageConfig {
            timer_resolution: Duration::from_millis(500),
            max_failures: 1,
            particle_ttl: Duration::from_secs(5),
            peer_id,
        };

        let pool: &ConnectionPoolApi = connectivity.as_ref();
        ScriptStorageBackend::new(pool.clone(), particle_failures_in, script_storage_config)
    };

    let pool_size = config.pool_size.unwrap_or(1);
    let pool_config = VmPoolConfig::new(pool_size, EXECUTION_TIMEOUT);

    std::fs::create_dir_all(tmp_dir).expect("create tmp dir");

    let startup_keypair = Keypair::generate_ed25519();
    let vm_config = vm_config(
        connectivity,
        script_storage_api,
        BaseVmConfig {
            peer_id,
            tmp_dir: tmp_dir.clone(),
            listen_on: listen_on.clone(),
            manager: m_id,
        },
        to_peer_id(&startup_keypair),
    );

    let mut node = Node::with(
        peer_id,
        swarm,
        network_api,
        script_storage_backend,
        vm_config,
        pool_config,
        particle_failures_out,
        None,
        "0.0.0.0:0".parse().unwrap(),
        bootstraps,
        startup_keypair,
    );

    node.listen(vec![listen_on]).expect("listen");

    (peer_id, node, management_kp, config)
}

pub fn create_swarm(config: SwarmConfig) -> (PeerId, Box<Node<AVM>>, Keypair, SwarmConfig) {
    create_swarm_with_runtime(config, aqua_vm_config)
}

pub fn create_memory_maddr() -> Multiaddr {
    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

pub fn create_tcp_maddr() -> Multiaddr {
    let port: u16 = 1000 + rand::thread_rng().gen_range(1, 3000);
    let mut maddr: Multiaddr = Protocol::Ip4("127.0.0.1".parse().unwrap()).into();
    maddr.push(Protocol::Tcp(port));
    maddr
}

pub fn test_module_cfg(name: &str) -> JValue {
    json!(
        {
            "name": name,
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            }
        }
    )
}

pub fn test_module_cfg_map(name: &str) -> HashMap<&str, JValue> {
    maplit::hashmap! {
        "module_name" => json!(name),
        "mem_pages_count" => json!(100),
        "logger_enabled" => json!(true),
        "preopened_files" => json!(["/tmp"]),
        "envs" => json!([]),
        "mapped_dirs" => json!([]),
        "mounted_binaries" => json!([]),
        "logging_mask" => JValue::Null,
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
}

pub async fn timeout<F, T>(dur: Duration, f: F) -> eyre::Result<T>
where
    F: std::future::Future<Output = T>,
{
    Ok(async_std::future::timeout(dur, f)
        .await
        .wrap_err(format!("timed out after {:?}", dur))?)
}

pub fn module_config(import_name: &str) -> JValue {
    json!(
        {
            "name": import_name,
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            }
        }
    )
}
