use super::defaults::*;
use crate::keys::{decode_key_pair, load_key_pair};
use crate::{BootstrapConfig, KademliaConfig};

use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;
use fluence_libp2p::{peerid_serializer, Transport};
use fs_utils::to_abs_path;
use particle_protocol::ProtocolConfig;

use derivative::Derivative;
use eyre::eyre;
use libp2p::core::Multiaddr;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::HashMap;
use std::net::IpAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::time::Duration;

#[serde_as]
#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct NodeConfig {
    #[serde(deserialize_with = "parse_or_load_root_keypair")]
    #[serde(default = "default_root_keypair")]
    #[derivative(Debug = "ignore")]
    pub root_key_pair: KeyPair,

    #[serde(deserialize_with = "parse_or_load_builtins_keypair")]
    #[serde(default = "default_builtins_keypair")]
    #[derivative(Debug = "ignore")]
    pub builtins_key_pair: KeyPair,

    /// Particle ttl for autodeploy
    #[serde(default = "default_particle_ttl")]
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

    /// Bootstrap nodes to join to the Fluence network
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// External multiaddresses to advertise; more flexible that IpAddr
    #[serde(default)]
    pub external_multiaddresses: Vec<Multiaddr>,

    #[serde(flatten)]
    pub prometheus_config: PrometheusConfig,

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
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub module_max_heap_size: Option<bytesize::ByteSize>,

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

    #[serde(default = "default_bootstrap_frequency")]
    pub bootstrap_frequency: usize,

    #[serde(default)]
    pub allow_local_addresses: bool,

    #[serde(default = "default_execution_timeout")]
    #[serde(with = "humantime_serde")]
    pub particle_execution_timeout: Duration,

    #[serde(default = "default_processing_timeout")]
    #[serde(with = "humantime_serde")]
    pub particle_processing_timeout: Duration,

    #[serde(with = "peerid_serializer")]
    #[serde(default = "default_management_peer_id")]
    pub management_peer_id: PeerId,
}

#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct TransportConfig {
    #[serde(default = "default_transport")]
    pub transport: Transport,

    /// Socket timeout for main transport
    #[serde(default = "default_socket_timeout")]
    #[serde(with = "humantime_serde")]
    pub socket_timeout: Duration,
}

#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct PrometheusConfig {
    #[serde(default = "default_prometheus_enabled")]
    pub prometheus_enabled: bool,

    /// Prometheus port
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,
}

#[derive(Clone, Deserialize, Derivative)]
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

#[derive(Clone, Deserialize, Debug, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PeerIdSerializable(#[serde(with = "peerid_serializer")] PeerId);
impl Deref for PeerIdSerializable {
    type Target = PeerId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PathOrValue {
    Value { value: String },
    Path { path: PathBuf },
}

#[derive(Deserialize)]
pub struct KeypairConfig {
    #[serde(default = "default_keypair_format")]
    pub format: String,
    #[serde(flatten)]
    pub keypair: Option<PathOrValue>,
    #[serde(default)]
    pub generate_on_absence: bool,
}

impl KeypairConfig {
    pub fn get_keypair(self, default: PathOrValue) -> Result<KeyPair, eyre::Report> {
        use crate::node_config::PathOrValue::{Path, Value};

        match self.keypair.unwrap_or(default) {
            Path { path } => {
                let path = to_abs_path(path);
                load_key_pair(path.clone(), self.format.clone(), self.generate_on_absence)
                    .map_err(|e| eyre!("Failed to load keypair from {:?}: {}", path, e))
            }
            Value { value } => decode_key_pair(value, self.format)
                .map_err(|e| eyre!("Failed to decode keypair: {}", e)),
        }
    }
}

fn parse_or_load_root_keypair<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    parse_or_load_keypair(deserializer, default_keypair_path())
}

fn parse_or_load_builtins_keypair<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    parse_or_load_keypair(deserializer, default_builtins_keypair_path())
}

/// Try to decode keypair from string as base58,
/// if failed – load keypair from file pointed at by same string
fn parse_or_load_keypair<'de, D>(
    deserializer: D,
    default_path: PathOrValue,
) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let config = KeypairConfig::deserialize(deserializer)?;
    config
        .get_keypair(default_path)
        .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))
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
