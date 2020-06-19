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
use ::config::{Config, File};
use clap::ArgMatches;
use failure::_core::str::FromStr;
use libp2p::{core::Multiaddr, identity::ed25519::PublicKey};
use std::{error::Error, net::IpAddr, time::Duration};
use trust_graph::KeyPair;

pub const WEBSOCKET_PORT: &str = "websocket-port";
pub const TCP_PORT: &str = "tcp-port";
pub const ROOT_KEY_PAIR_PATH: &str = "root-key-pair-path";
pub const ROOT_KEY_PAIR: &str = "root-key-pair";
pub const BOOTSTRAP_NODE: &str = "bootstrap-node";
pub const EXTERNAL_ADDR: &str = "external-ipaddr";
pub const CERTIFICATE_DIR: &str = "certificate-dir";
pub const CONFIG_FILE: &str = "config-file";
// Name of the roots section in config
pub const ROOTS: &str = "roots";

pub const DEFAULT_CERT_DIR: &str = "./.fluence/certificates";
pub const DEFAULT_KEY_DIR: &str = "./.fluence/secret_key";
pub const DEFAULT_CONFIG_FILE: &str = "./server/Config.toml";

#[derive(Deserialize, Debug)]
pub struct FluenceConfig {
    pub server_config: ServerConfig,
    /// Directory, where all certificates are stored.
    pub certificate_dir: String,
    /// Path to a secret key.
    pub root_key_pair: KeyPair,
    pub root_weights: Vec<(PublicKey, u32)>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ServerConfig {
    /// For TCP connections
    pub tcp_port: u16,

    /// Local ip address to listen on
    pub listen_ip: IpAddr,

    /// Socket timeout for main transport
    pub socket_timeout: Duration,

    /// Bootstrap nodes to join to the Fluence network
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// For ws connections
    pub websocket_port: u16,

    /// External address to advertise via identify protocol
    pub external_address: Option<IpAddr>,

    /// Prometheus port
    pub prometheus_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tcp_port: 7777,
            listen_ip: "0.0.0.0".parse().unwrap(),
            socket_timeout: Duration::from_secs(20),
            bootstrap_nodes: vec![],
            websocket_port: 9999,
            external_address: None,
            prometheus_port: 18080,
        }
    }
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

/// Build `FluenceConfig` by merging arguments and a config file.
/// Arguments have higher priority than config file.
#[allow(clippy::implicit_hasher)]
fn build_config(
    arguments: ArgMatches<'_>,
    config: Config,
) -> Result<FluenceConfig, Box<dyn Error>> {
    let mut server_config = ServerConfig::default();

    let merge_by_name = |name| {
        arguments
            .value_of(name)
            .map(|v| v.to_string())
            .or_else(|| config.get_str(name).ok())
    };

    if let Some(ws_port) = merge_by_name(WEBSOCKET_PORT) {
        let peer_port: u16 = u16::from_str(&ws_port)?;
        server_config.websocket_port = peer_port;
    }

    if let Some(tcp_port) = merge_by_name(TCP_PORT) {
        let node_port: u16 = u16::from_str(&tcp_port)?;
        server_config.tcp_port = node_port;
    }

    if let Some(bootstrap_node) = merge_by_name(BOOTSTRAP_NODE) {
        let bootstrap_node = Multiaddr::from_str(&bootstrap_node)?;
        server_config.bootstrap_nodes.push(bootstrap_node);
    };

    if let Some(external_address) = merge_by_name(EXTERNAL_ADDR) {
        let external_address = IpAddr::from_str(&external_address)?;
        server_config.external_address = Some(external_address);
    }

    let certificate_dir = merge_by_name(CERTIFICATE_DIR).unwrap_or_else(|| DEFAULT_CERT_DIR.into());

    let root_key_pair = {
        if let Some(key_pair_b58) = arguments.value_of(ROOT_KEY_PAIR) {
            decode_key_pair(key_pair_b58.to_string())?
        } else {
            let secret_key_path =
                merge_by_name(ROOT_KEY_PAIR_PATH).unwrap_or_else(|| DEFAULT_KEY_DIR.into());
            // TODO: it's not ok to CREATE key pair here, but it's OK to load it.
            load_or_create_key_pair(&secret_key_path)?
        }
    };

    let root_weights = load_weights(config);

    Ok(FluenceConfig {
        server_config,
        certificate_dir,
        root_key_pair,
        root_weights,
    })
}

fn load_weights(config: Config) -> Vec<(PublicKey, u32)> {
    fn parse_pk(s: String) -> PublicKey {
        let bytes = bs58::decode(s.clone()) // TODO: excess clone
            .into_vec()
            .unwrap_or_else(|_| panic!("Can't parse base58 from public key {}", s));

        PublicKey::decode(bytes.as_slice()).expect("Can't decode ed25519::PublicKey from bytes")
    }

    config
        .get_table(ROOTS)
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| v.into_int().map(|i: i64| (parse_pk(k), i as u32)))
        .collect::<Result<_, _>>()
        .expect("Can't parse weight to integer")
}

// loads config from arguments and a config file
pub fn load_config(arguments: ArgMatches<'_>) -> Result<FluenceConfig, Box<dyn Error>> {
    let config_file = arguments
        .value_of(CONFIG_FILE)
        .unwrap_or(DEFAULT_CONFIG_FILE);

    log::info!("Loading config from {}", config_file);
    let mut config = Config::default();
    config.merge(File::with_name(config_file).required(false))?;

    build_config(arguments, config)
}
