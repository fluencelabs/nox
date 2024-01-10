use std::collections::HashMap;
use std::net::IpAddr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use derivative::Derivative;
use eyre::eyre;
use fluence_keypair::KeyPair;
use libp2p::core::Multiaddr;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use fluence_libp2p::PeerId;
use fluence_libp2p::{peerid_serializer, Transport};
use fs_utils::to_abs_path;
use particle_protocol::ProtocolConfig;

use crate::keys::{decode_key, decode_secret_key, load_key};
use crate::system_services_config::{ServiceKey, SystemServicesConfig};
use crate::{BootstrapConfig, KademliaConfig};

use super::defaults::*;

#[serde_as]
#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct UnresolvedNodeConfig {
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

    /// Number of stepper VMs to create. By default, `num_cpus::get() * 2` is used
    #[serde(default = "default_aquavm_pool_size")]
    pub aquavm_pool_size: usize,

    /// Maximum heap size in bytes available for an interpreter instance.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub aquavm_max_heap_size: Option<bytesize::ByteSize>,

    /// Default heap size in bytes available for a WASM service unless otherwise specified.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub default_service_memory_limit: Option<bytesize::ByteSize>,

    #[serde(default)]
    pub kademlia: KademliaConfig,

    #[serde(default = "default_particle_queue_buffer_size")]
    pub particle_queue_buffer: usize,

    #[serde(default = "default_effects_queue_buffer_size")]
    pub effects_queue_buffer: usize,

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

    #[serde(with = "peerid_serializer")]
    #[serde(default = "default_management_peer_id")]
    pub management_peer_id: PeerId,

    #[serde(default = "default_allowed_binaries")]
    pub allowed_binaries: Vec<String>,

    #[serde(default)]
    pub system_services: SystemServicesConfig,

    #[serde(flatten)]
    pub chain_listener_config: Option<ChainListenerConfig>,
}

impl UnresolvedNodeConfig {
    pub fn resolve(mut self, base_dir: &Path) -> eyre::Result<NodeConfig> {
        self.load_system_services_envs();

        let bootstrap_nodes = match self.local {
            Some(true) => vec![],
            _ => self.bootstrap_nodes,
        };

        let root_key_pair = self
            .root_key_pair
            .unwrap_or_default()
            .get_keypair(default_keypair_path(base_dir))?;

        let builtins_key_pair = self
            .builtins_key_pair
            .unwrap_or_default()
            .get_keypair(default_builtins_keypair_path(base_dir))?;

        let mut allowed_binaries = self.allowed_binaries;
        allowed_binaries.push(self.system_services.aqua_ipfs.ipfs_binary_path.clone());
        allowed_binaries.push(self.system_services.connector.curl_binary_path.clone());

        let result = NodeConfig {
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
            aquavm_heap_size_limit: self.aquavm_max_heap_size,
            default_service_memory_limit: self.default_service_memory_limit,
            kademlia: self.kademlia,
            particle_queue_buffer: self.particle_queue_buffer,
            effects_queue_buffer: self.effects_queue_buffer,
            particle_processor_parallelism: self.particle_processor_parallelism,
            max_spell_particle_ttl: self.max_spell_particle_ttl,
            bootstrap_frequency: self.bootstrap_frequency,
            allow_local_addresses: self.allow_local_addresses,
            particle_execution_timeout: self.particle_execution_timeout,
            management_peer_id: self.management_peer_id,
            transport_config: self.transport_config,
            listen_config: self.listen_config,
            allowed_binaries,
            system_services: self.system_services,
            http_config: self.http_config,
            chain_listener_config: self.chain_listener_config,
        };

        Ok(result)
    }

    // This is a temporary solution to save backward compatibility for some time
    // Couldn't figure out how to use layered configs for this
    // Print warning not to forget to fix it in the future
    fn load_system_services_envs(&mut self) {
        if let Ok(aqua_ipfs_external_addr) =
            std::env::var("FLUENCE_ENV_AQUA_IPFS_EXTERNAL_API_MULTIADDR")
        {
            log::warn!(
                "Override configuration of aqua-ipfs system service (external multiaddr) from ENV"
            );
            self.system_services.aqua_ipfs.external_api_multiaddr = aqua_ipfs_external_addr;
        }

        if let Ok(aqua_ipfs_local_addr) = std::env::var("FLUENCE_ENV_AQUA_IPFS_LOCAL_API_MULTIADDR")
        {
            log::warn!(
                "Override configuration of aqua-ipfs system service (local multiaddr) from ENV"
            );
            self.system_services.aqua_ipfs.local_api_multiaddr = aqua_ipfs_local_addr;
        }

        if let Ok(enable_decider) = std::env::var("FLUENCE_ENV_CONNECTOR_JOIN_ALL_DEALS") {
            match enable_decider.as_str() {
                "true" => {
                    log::warn!(
                        "Override configuration of system services (enable decider) from ENV"
                    );
                    self.system_services.enable.push(ServiceKey::Decider);
                }
                "false" => {
                    log::warn!(
                        "Override configuration of system services (disable decider) from ENV"
                    );
                    self.system_services
                        .enable
                        .retain(|key| *key != ServiceKey::Decider);
                }
                _ => {}
            }
        }
        if let Ok(decider_api_endpoint) = std::env::var("FLUENCE_ENV_CONNECTOR_API_ENDPOINT") {
            log::warn!(
                "Override configuration of decider system spell (api endpoint) from ENV to {}",
                decider_api_endpoint
            );
            self.system_services.decider.network_api_endpoint = decider_api_endpoint;
        }

        if let Ok(decider_contract_addr) = std::env::var("FLUENCE_ENV_CONNECTOR_CONTRACT_ADDRESS") {
            log::warn!(
                "Override configuration of decider system spell (contract address) from ENV to  {}",
                decider_contract_addr
            );
            self.system_services.decider.matcher_address = decider_contract_addr;
        }

        if let Ok(decider_from_block) = std::env::var("FLUENCE_ENV_CONNECTOR_FROM_BLOCK") {
            log::warn!(
                "Override configuration of decider system spell (from block) from ENV to {}",
                decider_from_block
            );
            self.system_services.decider.start_block = decider_from_block;
        }

        if let Ok(decider_wallet_key) = std::env::var("FLUENCE_ENV_CONNECTOR_WALLET_KEY") {
            log::warn!("Override configuration of decider system spell (wallet key) from ENV");
            self.system_services.decider.wallet_key = Some(decider_wallet_key);
        }

        if let Ok(worker_ipfs_multiaddr) = std::env::var("FLUENCE_ENV_DECIDER_IPFS_MULTIADDR") {
            log::warn!(
                "Override configuration of decider system spell (ipfs multiaddr) from ENV to {}",
                worker_ipfs_multiaddr
            );
            self.system_services.decider.worker_ipfs_multiaddr = worker_ipfs_multiaddr;
        }

        if let Ok(worker_gas) = std::env::var("FLUENCE_ENV_CONNECTOR_WORKER_GAS") {
            match worker_gas.parse() {
                Ok(worker_gas) => {
                    log::warn!(
                        "Override configuration of decider system spell (worker gas) from ENV to {}", worker_gas
                    );
                    self.system_services.decider.worker_gas = worker_gas;
                }
                Err(err) => log::warn!(
                    "Unable to override worker gas, value is not a valid u64: {}",
                    err
                ),
            }
        }
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct NodeConfig {
    #[derivative(Debug = "ignore")]
    pub root_key_pair: KeyPair,

    #[derivative(Debug = "ignore")]
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

    /// Number of stepper VMs to create. By default, `num_cpus::get() * 2` is used
    pub aquavm_pool_size: usize,

    /// Maximum heap size in bytes available for an interpreter instance.
    pub aquavm_heap_size_limit: Option<bytesize::ByteSize>,

    /// Default heap size in bytes available for a WASM service unless otherwise specified.
    pub default_service_memory_limit: Option<bytesize::ByteSize>,

    pub kademlia: KademliaConfig,

    pub particle_queue_buffer: usize,

    pub effects_queue_buffer: usize,

    pub particle_processor_parallelism: Option<usize>,

    pub max_spell_particle_ttl: Duration,

    pub bootstrap_frequency: usize,

    pub allow_local_addresses: bool,

    pub particle_execution_timeout: Duration,

    pub management_peer_id: PeerId,

    pub allowed_binaries: Vec<String>,

    pub system_services: SystemServicesConfig,

    pub http_config: Option<HttpConfig>,

    pub chain_listener_config: Option<ChainListenerConfig>,
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
pub struct PeerIdSerializable(#[serde(with = "peerid_serializer")] PeerId);

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
pub struct ChainListenerConfig {
    pub ws_endpoint: String,
    pub cc_contract_address: String,
}
