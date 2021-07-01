use super::defaults::*;
use crate::keys::{decode_key_pair, load_key_pair};
use crate::{BootstrapConfig, KademliaConfig};

use fluence_identity::KeyPair;
use fluence_libp2p::peerid_serializer;
use fluence_libp2p::PeerId;
use particle_protocol::ProtocolConfig;

use derivative::Derivative;
use fs_utils::to_abs_path;
use libp2p::core::Multiaddr;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct NodeConfig {
    #[serde(deserialize_with = "parse_or_load_keypair")]
    #[derivative(Debug = "ignore")]
    pub root_key_pair: KeyPair,

    /// For TCP connections
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,

    /// Local ip address to listen on
    #[serde(default = "default_listen_ip")]
    pub listen_ip: IpAddr,

    /// Socket timeout for main transport
    #[serde(default = "default_socket_timeout")]
    #[serde(with = "humantime_serde")]
    pub socket_timeout: Duration,

    /// Bootstrap nodes to join to the Fluence network
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// For ws connections
    #[serde(default = "default_websocket_port")]
    pub websocket_port: u16,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// External multiaddresses to advertise; more flexible that IpAddr
    #[serde(default)]
    pub external_multiaddresses: Vec<Multiaddr>,

    /// Prometheus port
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,

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
    #[serde(default = "default_stepper_pool_size")]
    pub stepper_pool_size: usize,

    #[serde(default)]
    pub kademlia: KademliaConfig,

    #[serde(default = "default_particle_queue_buffer_size")]
    pub particle_queue_buffer: usize,
    #[serde(default = "default_particle_processor_parallelism")]
    pub particle_processor_parallelism: usize,

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
struct KeypairConfig {
    #[serde(default = "default_keypair_format")]
    format: String,
    #[serde(default = "default_keypair_path", flatten)]
    keypair: PathOrValue,
    #[serde(default = "bool::default")]
    generate_on_absence: bool,
}

/// Try to decode keypair from string as base58,
/// if failed – load keypair from file pointed at by same string
fn parse_or_load_keypair<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use crate::node_config::PathOrValue::{Path, Value};

    let result = KeypairConfig::deserialize(deserializer)?;

    match result.keypair {
        Path { path } => {
            let path = to_abs_path(path);
            load_key_pair(
                path.clone(),
                result.format.clone(),
                result.generate_on_absence,
            )
            .map_err(|e| {
                serde::de::Error::custom(format!("Failed to load keypair from {:?}: {}", path, e))
            })
        }
        Value { value } => decode_key_pair(value, result.format)
            .map_err(|e| serde::de::Error::custom(format!("Failed to decode keypair: {}", e))),
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
