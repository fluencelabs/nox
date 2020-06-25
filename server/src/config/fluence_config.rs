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
use clap::ArgMatches;
use fluence_faas::RawCoreModulesConfig;
use libp2p::core::Multiaddr;
use serde::Deserialize;
use std::collections::HashMap;
use std::{error::Error, net::IpAddr, time::Duration};
use trust_graph::{KeyPair, PublicKeyHashable};

pub const WEBSOCKET_PORT: &str = "websocket-port";
pub const TCP_PORT: &str = "tcp-port";
pub const ROOT_KEY_PAIR_PATH: &str = "root-key-pair-path";
pub const ROOT_KEY_PAIR: &str = "root-key-pair";
pub const BOOTSTRAP_NODE: &str = "bootstrap-node";
pub const EXTERNAL_ADDR: &str = "external-ipaddr";
pub const CERTIFICATE_DIR: &str = "certificate-dir";
pub const CONFIG_FILE: &str = "config-file";

pub const DEFAULT_CERT_DIR: &str = "./.fluence/certificates";
pub const DEFAULT_KEY_DIR: &str = "./.fluence/secret_key";
pub const DEFAULT_CONFIG_FILE: &str = "./server/Config.toml";

#[derive(Deserialize, Debug)]
pub struct FluenceConfig {
    #[serde(flatten)]
    pub server: ServerConfig,
    pub faas: RawCoreModulesConfig,
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
    load_or_create_key_pair(DEFAULT_KEY_DIR)
        .expect(format!("Failed to load keypair from {}", DEFAULT_KEY_DIR).as_str())
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

// loads config from arguments and a config file
pub fn load_config(arguments: ArgMatches<'_>) -> Result<FluenceConfig, Box<dyn Error>> {
    let config_file = arguments
        .value_of(CONFIG_FILE)
        .unwrap_or(DEFAULT_CONFIG_FILE);

    log::info!("Loading config from {}", config_file);

    let file_content = std::fs::read(config_file)?;
    let mut config: toml::value::Table = toml::from_slice(&file_content)?;
    for (k, v) in config.iter_mut() {
        if let Some(arg) = arguments.value_of(k) {
            *v = toml::value::Value::String(arg.to_string());
        }
    }

    let config = toml::value::Value::Table(config);
    let config = FluenceConfig::deserialize(config)?;

    Ok(config)
}
