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

use particle_node::Node;

use config_utils::{modules_dir, to_abs_path};
use fluence_client::Transport;
use fluence_libp2p::types::OneshotOutlet;
use fluence_libp2p::{build_memory_transport, build_transport};
use server_config::{BootstrapConfig, NetworkConfig, ServicesConfig};
use trust_graph::{Certificate, InMemoryStorage, TrustGraph};

use aquamarine::VmPoolConfig;
use async_std::task;
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use eyre::WrapErr;
use futures::{stream::iter, StreamExt};
use libp2p::{core::Multiaddr, identity::ed25519::Keypair, PeerId};
use rand::Rng;
use script_storage::ScriptStorageConfig;
use serde_json::{json, Value as JValue};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

/// Utility functions for tests.

pub type Result<T> = eyre::Result<T>;

/// In debug, VM startup time is big, account for that
#[cfg(debug_assertions)]
pub static TIMEOUT: Duration = Duration::from_secs(150);
#[cfg(not(debug_assertions))]
pub static TIMEOUT: Duration = Duration::from_secs(15);

pub static SHORT_TIMEOUT: Duration = Duration::from_millis(300);
pub static KAD_TIMEOUT: Duration = Duration::from_millis(500);

pub fn uuid() -> String {
    Uuid::new_v4().to_string()
}

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

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub fn enable_logs() {
    use log::LevelFilter::*;

    std::env::set_var("WASM_LOG", "info");

    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Debug)
        .filter(Some("aquamarine::actor"), Debug)
        .filter(Some("particle_node::bootstrapper"), Info)
        .filter(Some("yamux::connection::stream"), Info)
        .filter(Some("tokio_threadpool"), Info)
        .filter(Some("tokio_reactor"), Info)
        .filter(Some("mio"), Info)
        .filter(Some("tokio_io"), Info)
        .filter(Some("soketto"), Info)
        .filter(Some("yamux"), Info)
        .filter(Some("multistream_select"), Info)
        .filter(Some("libp2p_swarm"), Info)
        .filter(Some("libp2p_secio"), Info)
        .filter(Some("libp2p_websocket::framed"), Info)
        .filter(Some("libp2p_ping"), Info)
        .filter(Some("libp2p_core::upgrade::apply"), Info)
        .filter(Some("libp2p_kad::kbucket"), Info)
        .filter(Some("libp2p_kad"), Info)
        .filter(Some("libp2p_plaintext"), Info)
        .filter(Some("libp2p_identify::protocol"), Info)
        .filter(Some("cranelift_codegen"), Info)
        .filter(Some("wasmer_wasi"), Info)
        .filter(Some("wasmer_interface_types_fl"), Info)
        .filter(Some("async_std"), Info)
        .filter(Some("async_io"), Info)
        .filter(Some("polling"), Info)
        .filter(Some("cranelift_codegen"), Info)
        .filter(Some("walrus"), Info)
        .try_init()
        .ok();
}

#[derive(Debug)]
pub struct CreatedSwarm {
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
    // tmp dir, must be cleaned
    pub tmp_dir: PathBuf,
    // management_peer_id
    pub keypair: Keypair,
    // stop signal
    pub outlet: OneshotOutlet<()>,
}

pub fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(SwarmConfig::new(bs, maddr)),
        create_memory_maddr,
        true,
    )
}

pub fn make_swarms_with_cfg<F>(n: usize, update_cfg: F) -> Vec<CreatedSwarm>
where
    F: Fn(SwarmConfig) -> SwarmConfig,
{
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(update_cfg(SwarmConfig::new(bs, maddr))),
        create_memory_maddr,
        true,
    )
}

pub fn make_swarms_with<F, M>(
    n: usize,
    mut create_node: F,
    mut create_maddr: M,
    wait_connected: bool,
) -> Vec<CreatedSwarm>
where
    F: FnMut(Vec<Multiaddr>, Multiaddr) -> (PeerId, Box<Node>, PathBuf, Keypair),
    M: FnMut() -> Multiaddr,
{
    let addrs = (0..n).map(|_| create_maddr()).collect::<Vec<_>>();
    let nodes = addrs
        .iter()
        .map(|addr| {
            #[rustfmt::skip]
            let addrs = addrs.iter().filter(|&a| a != addr).cloned().collect::<Vec<_>>();
            let (id, node, tmp, m_kp) = create_node(addrs, addr.clone());
            ((id, addr.clone(), tmp, m_kp), node)
        })
        .collect::<Vec<_>>();

    let pools = iter(
        nodes
            .iter()
            .map(|(_, n)| n.network_api.connectivity())
            .collect::<Vec<_>>(),
    );
    let connected = pools.for_each_concurrent(None, |pool| async move {
        let pool = AsRef::<ConnectionPoolApi>::as_ref(&pool);
        let mut events = pool.lifecycle_events();
        loop {
            let num = pool.count_connections().await;
            if num >= n - 1 {
                break;
            }
            // wait until something changes
            events.next().await;
        }
    });

    // start all nodes
    let infos = nodes
        .into_iter()
        .map(|((peer_id, multiaddr, tmp_dir, keypair), node)| {
            let outlet = node.start();
            CreatedSwarm {
                peer_id,
                multiaddr,
                tmp_dir,
                keypair,
                outlet,
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

#[derive(Clone, Debug)]
pub struct SwarmConfig {
    pub bootstraps: Vec<Multiaddr>,
    pub listen_on: Multiaddr,
    pub trust: Option<Trust>,
    pub transport: Transport,
    pub tmp_dir: Option<PathBuf>,
    pub pool_size: Option<usize>,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            bootstraps: <_>::default(),
            listen_on: Multiaddr::empty(),
            trust: <_>::default(),
            transport: Transport::Memory,
            tmp_dir: <_>::default(),
            pool_size: <_>::default(),
        }
    }
}

impl SwarmConfig {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        Self {
            bootstraps,
            listen_on,
            transport: Transport::Memory,
            ..<_>::default()
        }
    }

    pub fn with_trust(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr, trust: Trust) -> Self {
        let mut this = Self::new(bootstraps, listen_on);
        this.trust = Some(trust);
        this
    }
}

pub fn create_swarm(config: SwarmConfig) -> (PeerId, Box<Node>, PathBuf, Keypair) {
    use libp2p::identity;

    #[rustfmt::skip]
    let SwarmConfig { bootstraps, listen_on, trust, transport, pool_size, .. } = config;

    let kp = Keypair::generate();
    let public_key = libp2p::identity::PublicKey::Ed25519(kp.public());
    let peer_id = PeerId::from(public_key);

    let management_kp = Keypair::generate();
    let m_public_key = libp2p::identity::PublicKey::Ed25519(management_kp.public());
    let m_id = PeerId::from(m_public_key);

    let tmp = config.tmp_dir.unwrap_or_else(make_tmp_dir);
    std::fs::create_dir_all(&tmp).expect("create tmp dir");
    let stepper_base_dir = tmp.join("stepper");
    let air_interpreter = put_aquamarine(modules_dir(&stepper_base_dir));

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

    // execution timeout
    let execution_timeout = Duration::from_secs(5);
    let pool_size = pool_size.unwrap_or(1);
    let pool_config = VmPoolConfig::new(
        peer_id,
        stepper_base_dir,
        air_interpreter,
        pool_size,
        execution_timeout,
    )
    .expect("create vm pool config");

    let services_config = ServicesConfig::new(peer_id, tmp.join("services"), <_>::default(), m_id)
        .expect("create services config");

    let network_config = NetworkConfig {
        key_pair: kp.clone(),
        local_peer_id: peer_id,
        trust_graph,
        bootstrap_nodes: bootstraps.clone(),
        bootstrap: BootstrapConfig::zero(),
        registry: None,
        protocol_config: Default::default(),
        kademlia_config: Default::default(),
        particle_queue_buffer: 100,
        particle_parallelism: 16,
        bootstrap_frequency: 1,
        allow_local_addresses: true,
        particle_timeout: Duration::from_secs(5),
    };

    use identity::Keypair::Ed25519;
    let transport = match transport {
        Transport::Memory => build_memory_transport(Ed25519(kp)),
        Transport::Network => build_transport(Ed25519(kp), Duration::from_secs(10)),
    };

    let script_storage_config = ScriptStorageConfig {
        timer_resolution: Duration::from_millis(500),
        max_failures: 1,
        particle_ttl: Duration::from_secs(5),
        peer_id,
    };

    let mut node = Node::with(
        peer_id,
        transport,
        services_config,
        pool_config,
        network_config,
        vec![listen_on.clone()],
        None,
        "0.0.0.0:0".parse().unwrap(),
        bootstraps,
        script_storage_config,
    )
    .expect("create node");

    node.listen(vec![listen_on]).expect("listen");

    (peer_id, node, tmp, management_kp)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

pub fn make_tmp_dir() -> PathBuf {
    use rand::distributions::Alphanumeric;

    let mut tmp = std::env::temp_dir();
    tmp.push("fluence_test/");
    let dir: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .collect();
    tmp.push(dir);

    std::fs::create_dir_all(&tmp).expect("create tmp dir");

    tmp
}

pub fn remove_dir(dir: &Path) {
    std::fs::remove_dir_all(&dir).unwrap_or_else(|_| panic!("remove dir {:?}", dir))
}

pub fn put_aquamarine(tmp: PathBuf) -> PathBuf {
    use air_interpreter_wasm::{INTERPRETER_WASM, VERSION};

    std::fs::create_dir_all(&tmp).expect("create tmp dir");

    let file = to_abs_path(tmp.join(format!("aquamarine_{}.wasm", VERSION)));
    std::fs::write(&file, INTERPRETER_WASM)
        .unwrap_or_else(|_| panic!("fs::write aquamarine.wasm to {:?}", file));

    file
}

pub fn load_module(path: &str, module_name: &str) -> Vec<u8> {
    let module = to_abs_path(PathBuf::from(path).join(format!("{}.wasm", module_name)));
    std::fs::read(&module).unwrap_or_else(|_| panic!("fs::read from {:?}", module))
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
