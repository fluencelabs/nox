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

use super::defaults::*;
use super::keys::{decode_key_pair, load_or_create_key_pair};
use crate::{BootstrapConfig, KademliaConfig, ListenConfig};

use config_utils::to_abs_path;
use fluence_identity::KeyPair;
use particle_protocol::ProtocolConfig;
use trust_graph::PublicKeyHashable;

use anyhow::{anyhow, Context};
use clap::{ArgMatches, Values};
use libp2p::core::{multiaddr::Protocol, Multiaddr};
use libp2p::PeerId;
use serde::Deserialize;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{collections::HashMap, net::IpAddr, path::PathBuf, time::Duration};

pub const WEBSOCKET_PORT: &str = "websocket_port";
pub const TCP_PORT: &str = "tcp_port";
pub const ROOT_KEY_PAIR: &str = "root_key_pair";
pub const BOOTSTRAP_NODE: &str = "bootstrap_nodes";
pub const EXTERNAL_ADDR: &str = "external_address";
pub const CERTIFICATE_DIR: &str = "certificate_dir";
pub const CONFIG_FILE: &str = "config_file";
pub const SERVICE_ENVS: &str = "service_envs";
pub const BLUEPRINT_DIR: &str = "blueprint_dir";
pub const MANAGEMENT_PEER_ID: &str = "management_peer_id";
pub const SERVICES_WORKDIR: &str = "services_workdir";
const ARGS: &[&str] = &[
    WEBSOCKET_PORT,
    TCP_PORT,
    ROOT_KEY_PAIR,
    BOOTSTRAP_NODE,
    EXTERNAL_ADDR,
    CERTIFICATE_DIR,
    CONFIG_FILE,
    SERVICE_ENVS,
    BLUEPRINT_DIR,
    MANAGEMENT_PEER_ID,
];

#[derive(Deserialize, Debug)]
pub struct FluenceConfig {
    #[serde(flatten)]
    pub server: NodeConfig,
    /// Directory, where all certificates are stored.
    #[serde(default = "default_cert_dir")]
    pub certificate_dir: String,

    // TODO: Need better UX for configuring root key pair.
    //       Currently if incorrect path is specified for root key pair, we silently create new keypair
    //       this may be undesired and unexpected for users.
    #[serde(deserialize_with = "parse_or_load_keypair", default = "load_key_pair")]
    pub root_key_pair: KeyPair,
}

#[derive(Clone, Deserialize, Debug)]
pub struct NodeConfig {
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

    pub root_weights: HashMap<PublicKeyHashable, u32>,

    /// Base directory for resources needed by application services
    #[serde(default = "default_services_basedir")]
    pub services_base_dir: PathBuf,

    #[serde(default)]
    #[serde(deserialize_with = "parse_envs")]
    pub services_envs: HashMap<Vec<u8>, Vec<u8>>,

    /// Base directory for resources needed by application services
    #[serde(default = "default_stepper_basedir")]
    pub stepper_base_dir: PathBuf,

    #[serde(default)]
    pub protocol_config: ProtocolConfig,

    /// Number of stepper VMs to create. By default, `num_cpus::get() * 2` is used
    #[serde(default = "default_stepper_pool_size")]
    pub stepper_pool_size: usize,

    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    #[serde(default = "default_air_interpreter_path")]
    pub air_interpreter_path: PathBuf,

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

    #[serde(deserialize_with = "parse_management_peer_id")]
    #[serde(default = "default_management_peer_id")]
    pub management_peer_id: PeerId,
}

impl NodeConfig {
    pub fn external_addresses(&self) -> Vec<Multiaddr> {
        let mut addrs = if let Some(external_address) = self.external_address {
            let external_tcp = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(self.tcp_port));
                maddr
            };

            let external_ws = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(self.websocket_port));
                maddr.push(Protocol::Ws("/".into()));
                maddr
            };

            vec![external_tcp, external_ws]
        } else {
            vec![]
        };

        addrs.extend(self.external_multiaddresses.iter().cloned());

        addrs
    }

    pub fn root_weights(&self) -> Vec<(fluence_identity::PublicKey, u32)> {
        self.root_weights
            .clone()
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect()
    }

    pub fn metrics_listen_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_ip, self.prometheus_port)
    }

    pub fn listen_config(&self) -> ListenConfig {
        ListenConfig {
            listen_ip: self.listen_ip,
            tcp_port: self.tcp_port,
            websocket_port: self.websocket_port,
        }
    }
}

/// Load keypair from default location
fn load_key_pair() -> KeyPair {
    load_or_create_key_pair(DEFAULT_KEY_DIR)
        .unwrap_or_else(|e| panic!("Failed to load keypair from {}: {:?}", DEFAULT_KEY_DIR, e))
}

/// Try to decode keypair from string as base58,
/// if failed – load keypair from file pointed at by same string
fn parse_or_load_keypair<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Either keypair encoded as base58 or a path where keypair is stored
    let bs58_or_path = String::deserialize(deserializer)?;
    if let Ok(keypair) = decode_key_pair(bs58_or_path.clone()) {
        Ok(keypair)
    } else {
        load_or_create_key_pair(&bs58_or_path).map_err(|e| {
            serde::de::Error::custom(format!(
                "Failed to load keypair from {}: {}",
                bs58_or_path, e
            ))
        })
    }
}

fn parse_management_peer_id<'de, D>(deserializer: D) -> Result<PeerId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let multihash = String::deserialize(deserializer)?;
    PeerId::from_str(&multihash).map_err(|err| {
        serde::de::Error::custom(format!(
            "Failed to deserialize management_peer_id {}: {}",
            multihash, err
        ))
    })
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

/// Take all command line arguments, and insert them into config appropriately
fn insert_args_to_config(
    arguments: ArgMatches<'_>,
    config: &mut toml::value::Table,
) -> anyhow::Result<()> {
    use toml::Value::*;

    fn single(mut value: Values<'_>) -> &str {
        value.next().unwrap()
    }

    fn multiple(value: Values<'_>) -> impl Iterator<Item = toml::Value> + '_ {
        value.map(|s| String(s.into()))
    }

    // Check each possible command line argument
    for &k in ARGS {
        let arg = match arguments.values_of(k) {
            Some(arg) => arg,
            None => continue,
        };

        // Convert value to a type of the corresponding field in `FluenceConfig`
        let value = match k {
            WEBSOCKET_PORT | TCP_PORT => Integer(single(arg).parse()?),
            BOOTSTRAP_NODE | SERVICE_ENVS => Array(multiple(arg).collect()),
            _ => String(single(arg).into()),
        };
        config.insert(k.to_string(), value);
    }

    Ok(())
}

fn validate_config(config: FluenceConfig) -> anyhow::Result<FluenceConfig> {
    let exists = config.server.air_interpreter_path.as_path().exists();
    let is_file = config.server.air_interpreter_path.is_file();
    if exists && !is_file {
        return Err(anyhow!(
            "Invalid path to air interpreter: {:?} is a directory, expected .wasm file",
            config.server.air_interpreter_path
        ));
    }
    if !exists {
        return Err(anyhow!(
            "Invalid path to air interpreter: path {:?} does not exists",
            config.server.air_interpreter_path
        ));
    }

    Ok(config)
}

// loads config from arguments and a config file
// TODO: avoid depending on ArgMatches
pub fn load_config(arguments: ArgMatches<'_>) -> anyhow::Result<FluenceConfig> {
    let config_file = arguments
        .value_of(CONFIG_FILE)
        .unwrap_or(DEFAULT_CONFIG_FILE);

    let config_file = to_abs_path(config_file.into());

    log::info!("Loading config from {:?}", config_file);

    let file_content =
        std::fs::read(&config_file).context(format!("Config wasn't found at {:?}", config_file))?;
    let config = deserialize_config(arguments, file_content)?;

    validate_config(config)
}

pub fn deserialize_config(
    arguments: ArgMatches<'_>,
    content: Vec<u8>,
) -> anyhow::Result<FluenceConfig> {
    let mut config: toml::value::Table =
        toml::from_slice(&content).context("deserializing config")?;

    insert_args_to_config(arguments, &mut config)?;

    let config = toml::value::Value::Table(config);
    let config = FluenceConfig::deserialize(config)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let config = r#"
            stepper_base_dir = "/stepper"
            stepper_module_name = "aquamarine"

            [root_weights]
            Ct8ewXqEzSUvLR9CVtW39tHEDu3iBRsj21DzBZMc8LB4 = 1
        "#;

        deserialize_config(<_>::default(), config.as_bytes().to_vec()).expect("deserialize config");
    }

    #[test]
    fn parse_default_config() {
        let config =
            std::fs::read("../../deploy/Config.default.toml").expect("find default config");
        let _config = deserialize_config(<_>::default(), config).expect("deserialize config");
    }

    #[test]
    fn duration() {
        let bs_config = BootstrapConfig::default();
        let s = toml::to_string(&bs_config).expect("serialize");
        println!("{}", s)
    }
}
