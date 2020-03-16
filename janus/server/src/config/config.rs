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

use clap::ArgMatches;
use failure::_core::str::FromStr;
use libp2p::core::Multiaddr;
use libp2p::identity::{ed25519, Keypair};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

pub const PEER_SERVICE_PORT: &str = "peer-service-port";
pub const NODE_SERVICE_PORT: &str = "node-service-port";
pub const SECRET_KEY: &str = "secret-key";
pub const PEER_SECRET_KEY: &str = "peer-secret-key";
pub const BOOTSTRAP_NODE: &str = "bootstrap-node";

pub struct JanusConfig {
    pub node_service_config: NodeServiceConfig,
    pub peer_service_config: PeerServiceConfig,
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

    /// Key that will be used during peer id creation.
    pub key_pair: Option<Keypair>,
}

impl Default for NodeServiceConfig {
    fn default() -> Self {
        Self {
            listen_port: 7777,
            listen_ip: "0.0.0.0".parse().unwrap(),
            socket_timeout: Duration::from_secs(20),
            bootstrap_nodes: vec![],
            key_pair: None,
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

    /// Key that will be used during peer id creation.
    pub key_pair: Option<Keypair>,
}

impl Default for PeerServiceConfig {
    fn default() -> Self {
        Self {
            listen_port: 9999,
            listen_ip: "0.0.0.0".parse().unwrap(),
            socket_timeout: Duration::from_secs(20),
            key_pair: None,
        }
    }
}

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

/// Generate config with merging arguments and a config file. Arguments are a higher priority then the config file.
#[allow(clippy::implicit_hasher)]
pub fn generate_config(
    arg_matches: ArgMatches,
    config_from_file: HashMap<String, String>,
) -> Result<JanusConfig, failure::Error> {
    let mut node_service_config = NodeServiceConfig::default();
    let mut peer_service_config = PeerServiceConfig::default();

    let merge_by_name = |name| {
        arg_matches
            .value_of(name)
            .or_else(|| config_from_file.get(name).map(|s| s.as_ref()))
    };

    if let Some(peer_port) = merge_by_name(PEER_SERVICE_PORT) {
        let peer_port: u16 = u16::from_str(peer_port)?;
        peer_service_config.listen_port = peer_port;
    }

    if let Some(node_port) = merge_by_name(NODE_SERVICE_PORT) {
        let node_port: u16 = u16::from_str(node_port)?;
        node_service_config.listen_port = node_port;
    }

    if let Some(secret_key_str) = merge_by_name(SECRET_KEY) {
        node_service_config.key_pair = Some(decode_key_pair(secret_key_str)?);
    }

    if let Some(secret_key_str) = merge_by_name(PEER_SECRET_KEY) {
        peer_service_config.key_pair = Some(decode_key_pair(secret_key_str)?);
    }

    if let Some(bootstrap_node) = merge_by_name(BOOTSTRAP_NODE) {
        let bootstrap_node = Multiaddr::from_str(bootstrap_node)?;
        node_service_config.bootstrap_nodes.push(bootstrap_node);
    };

    Ok(JanusConfig {
        node_service_config,
        peer_service_config,
    })
}
