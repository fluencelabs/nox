use std::collections::HashMap;
use std::net::IpAddr;
use std::ops::Deref;
use std::path::PathBuf;
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
use crate::system_services_config::SystemServicesConfig;
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

    /// Particle ttl for autodeploy
    #[serde(default = "default_auto_particle_ttl")]
    #[serde(with = "humantime_serde")]
    pub autodeploy_particle_ttl: Duration,

    /// Configure the number of ping attempts to check the readiness of the vm pool.
    /// Total wait time is the autodeploy_particle_ttl times the number of attempts.
    #[serde(default = "default_autodeploy_retry_attempts")]
    pub autodeploy_retry_attempts: u16,

    /// Affects builtins autodeploy. If set to true, then all builtins should be recreated and their state is cleaned up.
    #[serde(default)]
    pub force_builtins_redeploy: bool,

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

    #[serde(default)]
    pub bootstrap_config: BootstrapConfig,

    #[serde(default)]
    pub root_weights: HashMap<PeerIdSerializable, u32>,

    #[serde(default)]
    #[serde(deserialize_with = "parse_envs")]
    pub services_envs: HashMap<Vec<u8>, Vec<u8>>,

    #[serde(default)]
    pub protocol_config: ProtocolConfig,

    /// Number of stepper VMs to create. By default, `num_cpus::get() * 2` is used
    #[serde(default = "default_aquavm_pool_size")]
    pub aquavm_pool_size: usize,

    /// Maximum heap size in bytes available for an interpreter instance.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub aquavm_max_heap_size: Option<bytesize::ByteSize>,

    /// Maximum heap size in bytes available for a WASM module.
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default = "default_module_max_heap_size")]
    pub module_max_heap_size: bytesize::ByteSize,

    /// Default heap size in bytes available for a WASM module unless otherwise specified.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub module_default_heap_size: Option<bytesize::ByteSize>,

    #[serde(default)]
    pub kademlia: KademliaConfig,

    #[serde(default = "default_particle_queue_buffer_size")]
    pub particle_queue_buffer: usize,
    #[serde(default = "default_particle_processor_parallelism")]
    pub particle_processor_parallelism: Option<usize>,

    #[serde(default = "default_script_storage_timer_resolution")]
    pub script_storage_timer_resolution: Duration,

    #[serde(default = "default_script_storage_max_failures")]
    pub script_storage_max_failures: u8,

    #[serde(default = "default_script_storage_particle_ttl")]
    #[serde(with = "humantime_serde")]
    pub script_storage_particle_ttl: Duration,

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
    pub system_services_config: SystemServicesConfig,
}

impl UnresolvedNodeConfig {
    pub fn resolve(self) -> eyre::Result<NodeConfig> {
        let bootstrap_nodes = match self.local {
            Some(true) => vec![],
            _ => self.bootstrap_nodes,
        };

        let root_key_pair = self
            .root_key_pair
            .unwrap_or(KeypairConfig::default())
            .get_keypair(default_keypair_path())?;

        let builtins_key_pair = self
            .builtins_key_pair
            .unwrap_or(KeypairConfig::default())
            .get_keypair(default_builtins_keypair_path())?;

        let result = NodeConfig {
            bootstrap_nodes,
            root_key_pair,
            builtins_key_pair,
            external_address: self.external_address,
            external_multiaddresses: self.external_multiaddresses,
            metrics_config: self.metrics_config,
            bootstrap_config: self.bootstrap_config,
            root_weights: self.root_weights,
            services_envs: self.services_envs,
            protocol_config: self.protocol_config,
            aquavm_pool_size: self.aquavm_pool_size,
            aquavm_max_heap_size: self.aquavm_max_heap_size,
            module_max_heap_size: self.module_max_heap_size,
            module_default_heap_size: self.module_default_heap_size,
            kademlia: self.kademlia,
            particle_queue_buffer: self.particle_queue_buffer,
            particle_processor_parallelism: self.particle_processor_parallelism,
            script_storage_timer_resolution: self.script_storage_timer_resolution,
            script_storage_max_failures: self.script_storage_max_failures,
            script_storage_particle_ttl: self.script_storage_particle_ttl,
            max_spell_particle_ttl: self.max_spell_particle_ttl,
            bootstrap_frequency: self.bootstrap_frequency,
            allow_local_addresses: self.allow_local_addresses,
            particle_execution_timeout: self.particle_execution_timeout,
            management_peer_id: self.management_peer_id,

            autodeploy_particle_ttl: self.autodeploy_particle_ttl,
            autodeploy_retry_attempts: self.autodeploy_retry_attempts,
            force_builtins_redeploy: self.force_builtins_redeploy,
            transport_config: self.transport_config,
            listen_config: self.listen_config,
            allowed_binaries: self.allowed_binaries,
            system_services_config: self.system_services_config,
        };

        Ok(result)
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct NodeConfig {
    #[derivative(Debug = "ignore")]
    pub root_key_pair: KeyPair,

    #[derivative(Debug = "ignore")]
    pub builtins_key_pair: KeyPair,

    /// Particle ttl for autodeploy
    pub autodeploy_particle_ttl: Duration,

    /// Configure the number of ping attempts to check the readiness of the vm pool.
    /// Total wait time is the autodeploy_particle_ttl times the number of attempts.
    pub autodeploy_retry_attempts: u16,

    /// Affects builtins autodeploy. If set to true, then all builtins should be recreated and their state is cleaned up.
    pub force_builtins_redeploy: bool,

    pub transport_config: TransportConfig,

    pub listen_config: ListenConfig,

    /// Bootstrap nodes to join to the Fluence network
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// External multiaddresses to advertise; more flexible that IpAddr
    pub external_multiaddresses: Vec<Multiaddr>,

    pub metrics_config: MetricsConfig,

    pub bootstrap_config: BootstrapConfig,

    pub root_weights: HashMap<PeerIdSerializable, u32>,

    pub services_envs: HashMap<Vec<u8>, Vec<u8>>,

    pub protocol_config: ProtocolConfig,

    /// Number of stepper VMs to create. By default, `num_cpus::get() * 2` is used
    pub aquavm_pool_size: usize,

    /// Maximum heap size in bytes available for an interpreter instance.
    pub aquavm_max_heap_size: Option<bytesize::ByteSize>,

    /// Maximum heap size in bytes available for a WASM module.
    pub module_max_heap_size: bytesize::ByteSize,

    /// Default heap size in bytes available for a WASM module unless otherwise specified.
    pub module_default_heap_size: Option<bytesize::ByteSize>,

    pub kademlia: KademliaConfig,

    pub particle_queue_buffer: usize,

    pub particle_processor_parallelism: Option<usize>,

    pub script_storage_timer_resolution: Duration,

    pub script_storage_max_failures: u8,

    pub script_storage_particle_ttl: Duration,

    pub max_spell_particle_ttl: Duration,

    pub bootstrap_frequency: usize,

    pub allow_local_addresses: bool,

    pub particle_execution_timeout: Duration,

    pub management_peer_id: PeerId,

    pub allowed_binaries: Vec<String>,

    pub system_services_config: SystemServicesConfig,
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
}

#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,

    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    #[serde(default = "default_services_metrics_timer_resolution")]
    #[serde(with = "humantime_serde")]
    pub metrics_timer_resolution: Duration,

    #[serde(default = "default_max_builtin_metrics_storage_size")]
    pub max_builtin_metrics_storage_size: usize,
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

    #[serde(default)]
    pub listen_multiaddrs: Vec<Multiaddr>,
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

fn parse_envs<'de, D>(deserializer: D) -> Result<HashMap<Vec<u8>, Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let envs = HashMap::<String, String>::deserialize(deserializer)?;
    let envs = envs
        .into_iter()
        .map(|(k, v)| (k.into_bytes(), v.into_bytes()))
        .collect();

    Ok(envs)
}
