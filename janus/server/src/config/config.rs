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

use crate::key_storage;
use ::config::{Config, File};
use clap::ArgMatches;
use failure::_core::str::FromStr;
use libp2p::core::Multiaddr;
use libp2p::identity::{ed25519, ed25519::PublicKey, Keypair};
use std::error::Error;
use std::net::IpAddr;
use std::time::Duration;
use trust_graph::KeyPair;

pub const PEER_SERVICE_PORT: &str = "peer-service-port";
pub const NODE_SERVICE_PORT: &str = "node-service-port";
pub const ROOT_KEY_PAIR_PATH: &str = "root-key-pair-path";
pub const ROOT_KEY_PAIR: &str = "root-key-pair";
pub const BOOTSTRAP_NODE: &str = "bootstrap-node";
pub const CERTIFICATE_DIR: &str = "certificate-dir";
pub const CONFIG_FILE: &str = "config-file";
// Name of the roots section in config
pub const ROOTS: &str = "roots";

pub const DEFAULT_CERT_DIR: &str = "./.janus/certificates";
pub const DEFAULT_KEY_DIR: &str = "./.janus/secret_key";
pub const DEFAULT_CONFIG_FILE: &str = "./server/Config.toml";

pub struct JanusConfig {
    pub node_service_config: NodeServiceConfig,
    pub peer_service_config: PeerServiceConfig,
    /// Directory, where all certificates are stored.
    pub certificate_dir: String,
    /// Path to a secret key.
    pub root_key_pair: KeyPair,
    pub root_weights: Vec<(PublicKey, u32)>,
}

#[derive(Clone)]
pub struct NodeServiceConfig {
    /// Local port to listen on.
    pub listen_port: u16,

    /// Local ip address to listen on.
    pub listen_ip: IpAddr,

    /// Socket timeout for main transport.
    pub socket_timeout: Duration,

    /// Bootstrap nodes to join to the Fluence network.
    pub bootstrap_nodes: Vec<Multiaddr>,

    pub websocket_port: u16,
}

impl Default for NodeServiceConfig {
    fn default() -> Self {
        Self {
            listen_port: 7777,
            listen_ip: "0.0.0.0".parse().unwrap(),
            socket_timeout: Duration::from_secs(20),
            bootstrap_nodes: vec![],
            websocket_port: 9999,
        }
    }
}

#[derive(Clone)]
pub struct PeerServiceConfig {
    /// Local port to listen on.
    pub listen_port: u16,

    /// Local ip address to listen on.
    pub listen_ip: IpAddr,

    /// Socket timeout for main transport.
    pub socket_timeout: Duration,
}

impl Default for PeerServiceConfig {
    fn default() -> Self {
        Self {
            listen_port: 9999,
            listen_ip: "0.0.0.0".parse().unwrap(),
            socket_timeout: Duration::from_secs(20),
        }
    }
}

#[allow(dead_code)]
fn decode_key_pair(secret_key_str: &str) -> Result<Keypair, failure::Error> {
    let mut key_pair = base64::decode(secret_key_str)
        .map_err(|_| failure::err_msg("Secret key should be in base64 format."))?;
    let key_pair = key_pair.as_mut_slice();
    let key_pair = Keypair::Ed25519(
        ed25519::Keypair::decode(key_pair)
            .map_err(|_| failure::err_msg("Invalid secret key format."))?,
    );
    Ok(key_pair)
}

/// Build `JanusConfig` by merging arguments and a config file.
/// Arguments have higher priority than config file.
#[allow(clippy::implicit_hasher)]
fn build_config(arguments: ArgMatches, config: Config) -> Result<JanusConfig, Box<dyn Error>> {
    let mut node_service_config = NodeServiceConfig::default();
    let mut peer_service_config = PeerServiceConfig::default();

    let merge_by_name = |name| {
        arguments
            .value_of(name)
            .map(|v| v.to_string())
            .or_else(|| config.get_str(name).ok())
    };

    if let Some(peer_port) = merge_by_name(PEER_SERVICE_PORT) {
        let peer_port: u16 = u16::from_str(&peer_port)?;
        peer_service_config.listen_port = peer_port;
        node_service_config.websocket_port = peer_port; // TODO: remove peer service config
    }

    if let Some(node_port) = merge_by_name(NODE_SERVICE_PORT) {
        let node_port: u16 = u16::from_str(&node_port)?;
        node_service_config.listen_port = node_port;
    }

    if let Some(bootstrap_node) = merge_by_name(BOOTSTRAP_NODE) {
        let bootstrap_node = Multiaddr::from_str(&bootstrap_node)?;
        node_service_config.bootstrap_nodes.push(bootstrap_node);
    };

    let certificate_dir = merge_by_name(CERTIFICATE_DIR).unwrap_or_else(|| DEFAULT_CERT_DIR.into());

    let root_key_pair = {
        if let Some(key_pair_b58) = arguments.value_of(ROOT_KEY_PAIR) {
            key_storage::decode_key_pair(key_pair_b58.to_string())?
        } else {
            let secret_key_path =
                merge_by_name(ROOT_KEY_PAIR_PATH).unwrap_or_else(|| DEFAULT_KEY_DIR.into());
            // TODO: it's not ok to CREATE key pair here, but it's OK to load it.
            key_storage::load_or_create_key_pair(&secret_key_path)?
        }
    };

    let root_weights = load_weights(config);

    Ok(JanusConfig {
        node_service_config,
        peer_service_config,
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
pub fn load_config(arguments: ArgMatches) -> Result<JanusConfig, Box<dyn Error>> {
    let config_file = arguments
        .value_of(CONFIG_FILE)
        .unwrap_or(DEFAULT_CONFIG_FILE);

    println!("Loading config from {}", config_file);
    let mut config = Config::default();
    config.merge(File::with_name(config_file).required(true))?;

    build_config(arguments, config)
}
