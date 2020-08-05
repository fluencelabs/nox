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

use super::keys::{decode_key_pair, load_or_create_key_pair};
use crate::bootstrapper::BootstrapConfig;
use anyhow::Context;
use clap::{ArgMatches, Values};
use fluence_faas_service::RawModulesConfig;
use libp2p::core::Multiaddr;
use serde::Deserialize;
use std::collections::HashMap;
use std::{net::IpAddr, time::Duration};
use trust_graph::{KeyPair, PublicKeyHashable};

pub const WEBSOCKET_PORT: &str = "websocket_port";
pub const TCP_PORT: &str = "tcp_port";
pub const ROOT_KEY_PAIR: &str = "root_key_pair";
pub const BOOTSTRAP_NODE: &str = "bootstrap_nodes";
pub const EXTERNAL_ADDR: &str = "external_address";
pub const CERTIFICATE_DIR: &str = "certificate_dir";
pub const CONFIG_FILE: &str = "config_file";
pub const CORE_ENVS: &str = "core_envs";
const ARGS: &[&str] = &[
    WEBSOCKET_PORT,
    TCP_PORT,
    ROOT_KEY_PAIR,
    BOOTSTRAP_NODE,
    EXTERNAL_ADDR,
    CERTIFICATE_DIR,
    CONFIG_FILE,
    CORE_ENVS,
];

pub const DEFAULT_CERT_DIR: &str = "./.fluence/certificates";
pub const DEFAULT_KEY_DIR: &str = "./.fluence/secret_key";
pub const DEFAULT_CONFIG_FILE: &str = "./server/Config.toml";

#[derive(Deserialize, Debug)]
pub struct FluenceConfig {
    #[serde(flatten)]
    pub server: ServerConfig,
    pub faas: RawModulesConfig,
    /// Directory, where all certificates are stored.
    #[serde(default = "default_cert_dir")]
    pub certificate_dir: String,
    #[serde(deserialize_with = "parse_or_load_keypair", default = "load_key_pair")]
    pub root_key_pair: KeyPair,
    pub root_weights: HashMap<PublicKeyHashable, u32>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ServerConfig {
    /// For TCP connections
    #[serde(default = "default_tcp_port")]
    pub tcp_port: u16,

    /// Local ip address to listen on
    #[serde(default = "default_listen_ip")]
    pub listen_ip: IpAddr,

    /// Socket timeout for main transport
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: Duration,

    /// Bootstrap nodes to join to the Fluence network
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// For ws connections
    #[serde(default = "default_websocket_port")]
    pub websocket_port: u16,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// Prometheus port
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,

    #[serde(default)]
    pub bootstrap_config: BootstrapConfig,
}

impl ServerConfig {
    pub fn external_addresses(&self) -> Vec<Multiaddr> {
        use parity_multiaddr::Protocol;

        if let Some(external_address) = self.external_address {
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
        }
    }
}

fn default_tcp_port() -> u16 {
    7777
}
fn default_listen_ip() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}
fn default_socket_timeout() -> Duration {
    Duration::from_secs(20)
}
fn default_bootstrap_nodes() -> Vec<Multiaddr> {
    vec![]
}
fn default_websocket_port() -> u16 {
    9999
}
fn default_prometheus_port() -> u16 {
    18080
}
fn default_cert_dir() -> String {
    DEFAULT_CERT_DIR.into()
}

/// Load keypair from default location
fn load_key_pair() -> KeyPair {
    load_or_create_key_pair(DEFAULT_KEY_DIR).unwrap_or_else(|e| {
        panic!(format!(
            "Failed to load keypair from {}: {:?}",
            DEFAULT_KEY_DIR, e
        ))
    })
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

/// Take all command line arguments, and insert them into config appropriately
fn insert_args_to_config(
    arguments: ArgMatches<'_>,
    config: &mut toml::value::Table,
) -> anyhow::Result<()> {
    use toml::Value::*;

    /// Set given `envs` to each `module.wasi.envs` in `config.faas`
    fn set_core_envs(config: &mut toml::value::Table, arg: Values<'_>) -> Option<()> {
        // Path in config is: ["faas"]["module"][0]["wasi"]["envs"]
        let faas = config.get_mut("faas")?.as_table_mut()?;
        let core = faas.get_mut("module")?.as_array_mut()?;
        for module in core.iter_mut() {
            let wasi = module.get_mut("wasi")?.as_table_mut()?;
            let envs = wasi.get_mut("envs")?.as_array_mut()?;

            envs.extend(multiple(arg.clone()));
        }

        Some(())
    }

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

        match k {
            CORE_ENVS => {
                // Set envs for each core module
                set_core_envs(config, arg);
            }
            k => {
                // Convert value to a type of the corresponding field in `FluenceConfig`
                let value = match k {
                    WEBSOCKET_PORT | TCP_PORT => Integer(single(arg).parse()?),
                    BOOTSTRAP_NODE => Array(multiple(arg).collect()),
                    _ => String(single(arg).into()),
                };
                config.insert(k.to_string(), value);
            }
        }
    }

    Ok(())
}

// loads config from arguments and a config file
pub fn load_config(arguments: ArgMatches<'_>) -> anyhow::Result<FluenceConfig> {
    let config_file = arguments
        .value_of(CONFIG_FILE)
        .unwrap_or(DEFAULT_CONFIG_FILE);

    log::info!("Loading config from {}", config_file);

    let file_content =
        std::fs::read(config_file).context(format!("Config wasn't found at {}", config_file))?;
    let mut config: toml::value::Table = toml::from_slice(&file_content)?;

    insert_args_to_config(arguments, &mut config)?;

    let config = toml::value::Value::Table(config);
    let config = FluenceConfig::deserialize(config)?;

    Ok(config)
}
