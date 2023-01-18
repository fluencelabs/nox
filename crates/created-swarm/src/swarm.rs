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
use fluence_keypair::KeyPair;
use futures::{stream::iter, StreamExt};
use libp2p::core::multiaddr::Protocol;
use libp2p::{core::Multiaddr, PeerId};
use serde::Deserialize;

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::{AquaRuntime, VmConfig};
use aquamarine::{AquamarineApi, DataStoreError};
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use fluence_libp2p::random_multiaddr::{create_memory_maddr, create_tcp_maddr};
use fluence_libp2p::types::OneshotOutlet;
use fluence_libp2p::Transport;
use fs_utils::{create_dir, make_tmp_dir_peer_id, to_abs_path};
use particle_node::{Connectivity, Node};
use particle_protocol::ProtocolConfig;
use server_config::{default_script_storage_timer_resolution, BootstrapConfig, UnresolvedConfig};
use test_constants::{EXECUTION_TIMEOUT, KEEP_ALIVE_TIMEOUT, TRANSPORT_TIMEOUT};
use toy_vms::EasyVM;

#[allow(clippy::upper_case_acronyms)]
type AVM = aquamarine::AVM<DataStoreError>;

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
    #[derivative(Debug = "ignore")]
    pub connectivity: Connectivity,
    #[derivative(Debug = "ignore")]
    pub aquamarine_api: AquamarineApi,
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
        |bs, maddr| create_swarm_with_runtime(SwarmConfig::new(bs, maddr), |_| None),
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
        |bs, maddr| create_swarm_with_runtime(update_cfg(SwarmConfig::new(bs, maddr)), |_| delay),
        create_memory_maddr,
        bootstraps,
        true,
    )
}

pub fn make_swarms_with_keypair(
    n: usize,
    keypair: KeyPair,
    spell_base_dir: Option<String>,
) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        cfg.keypair = keypair.clone();
        cfg.spell_base_dir = spell_base_dir.clone().map(PathBuf::from);
        cfg
    })
}

pub fn make_swarms_with_builtins(
    n: usize,
    path: &Path,
    keypair: Option<KeyPair>,
    spell_base_dir: Option<String>,
) -> Vec<CreatedSwarm> {
    make_swarms_with_cfg(n, |mut cfg| {
        if let Some(keypair) = &keypair {
            cfg.keypair = keypair.clone();
        }
        cfg.builtins_dir = Some(to_abs_path(path.into()));
        cfg.spell_base_dir = spell_base_dir.clone().map(PathBuf::from);
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
            let aquamarine_api = node.aquamarine_api.clone();
            let outlet = node.start().expect("node start");

            CreatedSwarm {
                peer_id,
                multiaddr: config.listen_on,
                tmp_dir: config.tmp_dir.unwrap(),
                management_keypair,
                outlet,
                connectivity,
                aquamarine_api,
            }
        })
        .collect();

    if wait_connected {
        task::block_on(connected);
    }

    infos
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
    pub tmp_dir: Option<PathBuf>,
    pub pool_size: Option<usize>,
    pub builtins_dir: Option<PathBuf>,
    pub spell_base_dir: Option<PathBuf>,
    pub timer_resolution: Duration,
}

impl SwarmConfig {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        let transport = match listen_on.iter().next() {
            Some(Protocol::Memory(_)) => Transport::Memory,
            _ => Transport::Network,
        };
        Self {
            keypair: fluence_keypair::KeyPair::generate_ed25519(),
            builtins_keypair: fluence_keypair::KeyPair::generate_ed25519(),
            bootstraps,
            listen_on,
            transport,
            tmp_dir: None,
            pool_size: <_>::default(),
            builtins_dir: None,
            spell_base_dir: None,
            timer_resolution: default_script_storage_timer_resolution(),
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

    let avm_base_dir = tmp_dir.join("interpreter");

    VmConfig::new(peer_id, avm_base_dir, air_interpreter, None)
}

pub fn create_swarm_with_runtime<RT: AquaRuntime>(
    mut config: SwarmConfig,
    vm_config: impl Fn(BaseVmConfig) -> RT::Config,
) -> (PeerId, Box<Node<RT>>, KeyPair, SwarmConfig) {
    use serde_json::json;

    let format = match &config.keypair {
        KeyPair::Ed25519(_) => "ed25519",
        KeyPair::Rsa(_) => "rsa",
        KeyPair::Secp256k1(_) => "secp256k1",
    };

    let peer_id = libp2p::identity::Keypair::from(config.keypair.clone())
        .public()
        .to_peer_id();

    if config.tmp_dir.is_none() {
        config.tmp_dir = Some(make_tmp_dir_peer_id(peer_id.to_string()));
    }
    let tmp_dir = config.tmp_dir.as_ref().unwrap();
    create_dir(tmp_dir).expect("create tmp dir");

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
    });

    let node_config: UnresolvedConfig =
        UnresolvedConfig::deserialize(node_config).expect("created_swarm: deserialize config");

    let mut resolved = node_config.resolve().expect("failed to resolve config");
    create_dir(&resolved.dir_config.builtins_base_dir).expect("create builtins dir");

    resolved.node_config.transport_config.transport = Transport::Memory;
    resolved.node_config.transport_config.socket_timeout = TRANSPORT_TIMEOUT;
    resolved.node_config.protocol_config =
        ProtocolConfig::new(TRANSPORT_TIMEOUT, KEEP_ALIVE_TIMEOUT, TRANSPORT_TIMEOUT);

    resolved.node_config.bootstrap_nodes = config.bootstraps.clone();
    resolved.node_config.bootstrap_config = BootstrapConfig::zero();
    resolved.node_config.bootstrap_frequency = 1;

    resolved.metrics_config.metrics_enabled = false;

    resolved.node_config.allow_local_addresses = true;

    resolved.node_config.aquavm_pool_size = config.pool_size.unwrap_or(1);
    resolved.node_config.particle_execution_timeout = EXECUTION_TIMEOUT;

    resolved.node_config.script_storage_timer_resolution = config.timer_resolution;

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
    let mut node = Node::new(resolved, vm_config, "some version").expect("create node");
    node.listen(vec![config.listen_on.clone()]).expect("listen");

    (
        node.key_manager.get_host_peer_id(),
        node,
        management_kp,
        config,
    )
}

pub fn create_swarm(config: SwarmConfig) -> (PeerId, Box<Node<AVM>>, KeyPair, SwarmConfig) {
    create_swarm_with_runtime(config, aqua_vm_config)
}
