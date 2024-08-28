/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, Ipv4Addr};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use cid_utils::Hash;
use clarity::PrivateKey;
use core_distributor::CoreRange;
use derivative::Derivative;
use eyre::eyre;
use fluence_keypair::KeyPair;
use libp2p::core::Multiaddr;
use libp2p::swarm::InvalidProtocol;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use fluence_libp2p::PeerId;
use fluence_libp2p::Transport;
use fs_utils::to_abs_path;
use hex_utils::serde_as::Hex;
use particle_protocol::ProtocolConfig;
use types::peer_id;

use crate::avm_config::AVMConfig;
use crate::kademlia_config::{KademliaConfig, UnresolvedKademliaConfig};
use crate::keys::{decode_key, decode_secret_key, load_key};
use crate::services_config::ServicesConfig;
use crate::system_services_config::SystemServicesConfig;
use crate::BootstrapConfig;

use super::defaults::*;

#[serde_as]
#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct UnresolvedNodeConfig {
    #[serde(default = "default_cpus_range")]
    pub cpus_range: Option<CoreRange>,

    #[serde(default = "default_system_cpu_count")]
    pub system_cpu_count: usize,

    #[derivative(Debug = "ignore")]
    pub root_key_pair: Option<KeypairConfig>,

    #[derivative(Debug = "ignore")]
    #[serde(default)]
    pub builtins_key_pair: Option<KeypairConfig>,

    #[serde(flatten)]
    pub transport_config: TransportConfig,

    #[serde(flatten)]
    pub listen_config: ListenConfig,

    #[serde(default)]
    local: Option<bool>,
    /// Bootstrap nodes to join to the Fluence network
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// External multiaddresses to advertise; more flexible that IpAddr
    #[serde(default)]
    pub external_multiaddresses: Vec<Multiaddr>,

    #[serde(flatten)]
    pub metrics_config: MetricsConfig,

    #[serde(flatten)]
    pub health_config: HealthConfig,

    #[serde(flatten)]
    pub http_config: Option<HttpConfig>,

    #[serde(default)]
    pub bootstrap_config: BootstrapConfig,

    #[serde(default)]
    pub root_weights: HashMap<PeerIdSerializable, u32>,

    #[serde(default)]
    pub services_envs: HashMap<String, String>,

    #[serde(default)]
    pub protocol_config: ProtocolConfig,

    /// These are the AquaVM limits that are used by the AquaVM limit check.
    #[derivative(Debug = "ignore")]
    pub avm_config: Option<AVMConfig>,

    /// Number of AVMs to create. By default, `num_cpus::get() * 2` is used
    #[serde(default = "default_aquavm_pool_size")]
    pub aquavm_pool_size: usize,

    /// Default heap size in bytes available for a WASM service unless otherwise specified.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub default_service_memory_limit: Option<bytesize::ByteSize>,

    #[serde(default)]
    pub kademlia: UnresolvedKademliaConfig,

    #[serde(default = "default_particle_queue_buffer_size")]
    pub particle_queue_buffer: usize,

    #[serde(default = "default_effects_queue_buffer_size")]
    pub effects_queue_buffer: usize,

    #[serde(default = "default_workers_queue_buffer_size")]
    pub workers_queue_buffer: usize,

    #[serde(default = "default_particle_processor_parallelism")]
    pub particle_processor_parallelism: Option<usize>,

    #[serde(default = "default_max_spell_particle_ttl")]
    #[serde(with = "humantime_serde")]
    pub max_spell_particle_ttl: Duration,

    #[serde(default = "default_bootstrap_frequency")]
    pub bootstrap_frequency: usize,

    #[serde(default)]
    pub allow_local_addresses: bool,

    #[serde(default = "default_execution_timeout")]
    #[serde(with = "humantime_serde")]
    pub particle_execution_timeout: Duration,

    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    #[serde(default = "default_management_peer_id")]
    pub management_peer_id: PeerId,

    // TODO: leave for now to migrate
    #[serde(default = "default_allowed_binaries")]
    pub allowed_binaries: Vec<String>,

    #[serde(default = "default_effectors_config")]
    pub effectors: EffectorsConfig,

    #[serde(default)]
    pub system_services: SystemServicesConfig,

    pub chain_config: Option<ChainConfig>,

    pub chain_listener_config: Option<ChainListenerConfig>,

    #[serde(default = "default_dev_mode_config")]
    pub dev_mode: DevModeConfig,

    #[serde(default)]
    pub services: ServicesConfig,

    #[serde(default)]
    pub network: Network,

    pub vm: Option<VmConfig>,
}

#[serde_as]
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Dar,
    Stage,
    Kras,
    Custom(#[serde_as(as = "Hex")] [u8; 32]),
}

impl TryFrom<&Network> for StreamProtocol {
    type Error = InvalidProtocol;

    fn try_from(value: &Network) -> Result<Self, Self::Error> {
        let id = match value {
            Network::Dar => "dar".to_string(),
            Network::Stage => "stage".to_string(),
            Network::Kras => "kras".to_string(),
            Network::Custom(bytes) => hex::encode(bytes).to_lowercase(),
        };

        let kad_protocol_name = format!("/fluence/kad/{}/1.0.0", id);
        StreamProtocol::try_from_owned(kad_protocol_name.clone())
    }
}

impl UnresolvedNodeConfig {
    pub fn resolve(self, persistent_base_dir: &Path) -> eyre::Result<NodeConfig> {
        let bootstrap_nodes = match self.local {
            Some(true) => vec![],
            _ => self.bootstrap_nodes,
        };

        let root_key_pair = self
            .root_key_pair
            .unwrap_or_default()
            .get_keypair(default_keypair_path(persistent_base_dir))?;

        let builtins_key_pair = self
            .builtins_key_pair
            .unwrap_or_default()
            .get_keypair(default_builtins_keypair_path(persistent_base_dir))?;

        let allowed_effectors = self
            .effectors
            .0
            .into_values()
            .map(|effector_config| {
                let wasm_cid = effector_config.wasm_cid;
                let allowed_binaries = effector_config.allowed_binaries;
                (wasm_cid, allowed_binaries)
            })
            .collect::<_>();

        let cpus_range = self.cpus_range.unwrap_or_default();

        let kademlia = self.kademlia.resolve(&self.network)?;

        let result = NodeConfig {
            system_cpu_count: self.system_cpu_count,
            cpus_range,
            bootstrap_nodes,
            root_key_pair,
            builtins_key_pair,
            external_address: self.external_address,
            external_multiaddresses: self.external_multiaddresses,
            metrics_config: self.metrics_config,
            health_config: self.health_config,
            bootstrap_config: self.bootstrap_config,
            root_weights: self.root_weights,
            services_envs: self.services_envs,
            protocol_config: self.protocol_config,
            aquavm_pool_size: self.aquavm_pool_size,
            default_service_memory_limit: self.default_service_memory_limit,
            avm_config: self.avm_config.unwrap_or_default(),
            kademlia,
            particle_queue_buffer: self.particle_queue_buffer,
            effects_queue_buffer: self.effects_queue_buffer,
            workers_queue_buffer: self.workers_queue_buffer,
            particle_processor_parallelism: self.particle_processor_parallelism,
            max_spell_particle_ttl: self.max_spell_particle_ttl,
            bootstrap_frequency: self.bootstrap_frequency,
            allow_local_addresses: self.allow_local_addresses,
            particle_execution_timeout: self.particle_execution_timeout,
            management_peer_id: self.management_peer_id,
            transport_config: self.transport_config,
            listen_config: self.listen_config,
            allowed_effectors,
            dev_mode_config: self.dev_mode,
            system_services: self.system_services,
            http_config: self.http_config,
            chain_config: self.chain_config,
            chain_listener_config: self.chain_listener_config,
            services: self.services,
            network: self.network,
            vm: self.vm,
        };

        Ok(result)
    }
}

#[derive(Clone, Derivative, Serialize)]
#[derivative(Debug)]
pub struct NodeConfig {
    pub cpus_range: CoreRange,

    pub system_cpu_count: usize,

    #[derivative(Debug = "ignore")]
    #[serde(skip)]
    pub root_key_pair: KeyPair,

    #[derivative(Debug = "ignore")]
    #[serde(skip)]
    pub builtins_key_pair: KeyPair,

    pub transport_config: TransportConfig,

    pub listen_config: ListenConfig,

    /// Bootstrap nodes to join to the Fluence network
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// External multiaddresses to advertise; more flexible that IpAddr
    pub external_multiaddresses: Vec<Multiaddr>,

    pub metrics_config: MetricsConfig,

    pub health_config: HealthConfig,

    pub bootstrap_config: BootstrapConfig,

    pub root_weights: HashMap<PeerIdSerializable, u32>,

    pub services_envs: HashMap<String, String>,

    pub protocol_config: ProtocolConfig,

    /// Number of AVMs to create. By default, `num_cpus::get() * 2` is used
    pub aquavm_pool_size: usize,

    /// Default heap size in bytes available for a WASM service unless otherwise specified.
    pub default_service_memory_limit: Option<bytesize::ByteSize>,

    /// These are the AquaVM limits that are used by the AquaVM limit check.
    pub avm_config: AVMConfig,

    pub kademlia: KademliaConfig,

    pub particle_queue_buffer: usize,

    pub effects_queue_buffer: usize,

    pub workers_queue_buffer: usize,

    pub particle_processor_parallelism: Option<usize>,

    pub max_spell_particle_ttl: Duration,

    pub bootstrap_frequency: usize,

    pub allow_local_addresses: bool,

    pub particle_execution_timeout: Duration,

    #[serde(serialize_with = "peer_id::serde::serialize")]
    pub management_peer_id: PeerId,

    pub allowed_effectors: HashMap<Hash, HashMap<String, PathBuf>>,

    pub dev_mode_config: DevModeConfig,

    pub system_services: SystemServicesConfig,

    pub http_config: Option<HttpConfig>,

    pub chain_config: Option<ChainConfig>,

    pub chain_listener_config: Option<ChainListenerConfig>,

    pub services: ServicesConfig,

    pub network: Network,

    pub vm: Option<VmConfig>,
}

#[derive(Clone, Deserialize, Serialize, Derivative, Copy)]
#[derivative(Debug)]
pub struct TransportConfig {
    #[serde(default = "default_transport")]
    pub transport: Transport,

    /// Socket timeout for main transport
    #[serde(default = "default_socket_timeout")]
    #[serde(with = "humantime_serde")]
    pub socket_timeout: Duration,

    pub max_pending_incoming: Option<u32>,

    pub max_pending_outgoing: Option<u32>,

    pub max_established_incoming: Option<u32>,

    pub max_established_outgoing: Option<u32>,

    #[serde(default = "default_max_established_per_peer_limit")]
    pub max_established_per_peer: Option<u32>,

    pub max_established: Option<u32>,

    #[serde(with = "humantime_serde")]
    #[serde(default = "default_connection_idle_timeout")]
    pub connection_idle_timeout: Duration,
}

#[derive(Clone, Deserialize, Serialize, Derivative, Copy)]
#[derivative(Debug)]
pub struct HttpConfig {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,

    #[serde(default = "default_services_metrics_timer_resolution")]
    #[serde(with = "humantime_serde")]
    pub metrics_timer_resolution: Duration,

    #[serde(default = "default_max_builtin_metrics_storage_size")]
    pub max_builtin_metrics_storage_size: usize,

    #[serde(default = "default_tokio_metrics_enabled")]
    pub tokio_metrics_enabled: bool,

    #[serde(default = "default_tokio_metrics_poll_histogram_enabled")]
    pub tokio_metrics_poll_histogram_enabled: bool,
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct HealthConfig {
    #[serde(default = "default_health_check_enabled")]
    pub health_check_enabled: bool,
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct ListenConfig {
    /// For TCP connections
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,

    /// Local ip address to listen on
    #[serde(default = "default_listen_ip")]
    pub listen_ip: IpAddr,

    /// For ws connections
    #[serde(default = "default_websocket_port")]
    pub websocket_port: u16,
}

#[derive(Clone, Deserialize, Serialize, Debug, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PeerIdSerializable(
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    PeerId,
);

impl Deref for PeerIdSerializable {
    type Target = PeerId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum PathOrValue {
    Value { value: String },
    Path { path: PathBuf },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KeypairConfig {
    #[serde(default = "default_key_format")]
    pub format: String,
    #[serde(flatten)]
    #[serde(default)]
    pub keypair: Option<PathOrValue>,
    #[serde(default)]
    pub secret_key: Option<String>,
    #[serde(default)]
    pub generate_on_absence: bool,
}

impl Default for KeypairConfig {
    fn default() -> Self {
        Self {
            format: default_key_format(),
            keypair: None,
            secret_key: None,
            generate_on_absence: true,
        }
    }
}

impl KeypairConfig {
    pub fn get_keypair(self, default: PathOrValue) -> Result<KeyPair, eyre::Report> {
        use crate::node_config::PathOrValue::{Path, Value};

        debug_assert!(
            !(self.secret_key.is_some() && self.keypair.is_some()),
            "shouldn't have both secret_key and keypair defined in KeypairConfig"
        );

        // first, try to load secret key
        if let Some(secret_key) = self.secret_key {
            let secret_key = base64
                .decode(secret_key)
                .map_err(|err| eyre!("base64 decoding failed: {}", err))?;
            return decode_secret_key(secret_key.clone(), self.format.clone())
                .map_err(|e| eyre!("Failed to decode secret key from {:?}: {}", secret_key, e));
        }

        // if there's no secret key, try to load keypair
        match self.keypair.unwrap_or(default) {
            Path { path } => {
                let path = to_abs_path(path);
                load_key(path.clone(), self.format.clone(), self.generate_on_absence)
                    .map_err(|e| eyre!("Failed to load secret key from {:?}: {}", path, e))
            }
            Value { value } => decode_key(value, self.format),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct ChainConfig {
    pub http_endpoint: String,
    pub diamond_contract_address: String,
    pub network_id: u64,
    pub wallet_key: PrivateKey,
    /// If none, comes from the chain
    pub default_base_fee: Option<u64>,
    /// If none, comes from the chain
    pub default_priority_fee: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct ChainListenerConfig {
    pub ws_endpoint: String,
    pub ccp_endpoint: Option<String>,
    /// How often to poll proofs
    #[serde(default = "default_proof_poll_period")]
    #[serde(with = "humantime_serde")]
    pub proof_poll_period: Duration, // TODO: must be >0
    /// Min number of proofs in a batch for CU to be sent on chain
    #[serde(default = "default_min_batch_count")]
    pub min_batch_count: usize,
    /// Max number of proofs in a batch for CU to be sent on chain
    #[serde(default = "default_max_batch_count")]
    pub max_batch_count: usize,
    /// Max number of proofs batches to be sent on chain
    #[serde(default = "default_max_proof_batch_size")]
    pub max_proof_batch_size: usize,
    /// Defined how much time before epoch end to send all found proofs in any batches
    #[serde(default = "default_epoch_end_window")]
    #[serde(with = "humantime_serde")]
    pub epoch_end_window: Duration,
}

/// Name of the effector module
/// Current is used only for users and is ignored by Nox
type EffectorModuleName = String;

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct EffectorsConfig(HashMap<EffectorModuleName, EffectorConfig>);

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct EffectorConfig {
    #[derivative(Debug(format_with = "std::fmt::Display::fmt"))]
    wasm_cid: Hash,
    allowed_binaries: HashMap<String, PathBuf>,
}

fn default_effectors_config() -> EffectorsConfig {
    let config = default_effectors()
        .into_iter()
        .map(|(module_name, config)| {
            (
                module_name,
                EffectorConfig {
                    wasm_cid: Hash::from_string(&config.0).unwrap(),
                    allowed_binaries: config.1,
                },
            )
        })
        .collect::<_>();
    EffectorsConfig(config)
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct DevModeConfig {
    #[serde(default)]
    pub enable: bool,
    /// Mounted binaries mapping: binary name (used in the effector modules) to binary path
    #[serde(default = "default_binaries_mapping")]
    pub binaries: BTreeMap<String, PathBuf>,
}

fn default_dev_mode_config() -> DevModeConfig {
    DevModeConfig {
        enable: false,
        binaries: default_binaries_mapping(),
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct VmConfig {
    #[serde(default = "default_libvirt_uri")]
    pub libvirt_uri: String,
    #[serde(default)]
    pub allow_gpu: bool,
    pub network: VmNetworkConfig,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct VmNetworkConfig {
    #[serde(default = "default_bridge_name")]
    pub bridge_name: String,
    pub public_ip: Ipv4Addr,
    #[serde(default = "default_vm_ip")]
    pub vm_ip: Ipv4Addr,
    #[serde(default = "default_host_ssh_port")]
    pub host_ssh_port: u16,
    #[serde(default = "default_vm_ssh_port")]
    pub vm_ssh_port: u16,
    #[serde(default = "default_port_range_config")]
    pub port_range: PortRangeConfig,
}

fn default_libvirt_uri() -> String {
    String::from("qemu:///system")
}

fn default_bridge_name() -> String {
    String::from("virbr0")
}

fn default_host_ssh_port() -> u16 {
    2222
}

fn default_vm_ssh_port() -> u16 {
    22
}

fn default_vm_ip() -> Ipv4Addr {
    Ipv4Addr::new(192, 168, 122, 112)
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct PortRangeConfig {
    pub start: u16,
    pub end: u16,
}

fn default_port_range_config() -> PortRangeConfig {
    PortRangeConfig {
        start: 1000,
        end: 65535,
    }
}
