/*
 * Copyright 2023 Fluence Labs Limited
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
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceKey {
    AquaIpfs,
    TrustGraph,
    Registry,
    Decider,
}

impl ServiceKey {
    pub fn all_values() -> Vec<Self> {
        vec![
            Self::AquaIpfs,
            Self::TrustGraph,
            Self::Registry,
            Self::Decider,
        ]
    }

    pub fn from_string(name: &str) -> Option<ServiceKey> {
        serde_json::from_str::<ServiceKey>(name).ok()
    }
}

impl std::fmt::Display for ServiceKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AquaIpfs => write!(f, "aqua-ipfs"),
            Self::TrustGraph => write!(f, "trust-graph"),
            Self::Registry => write!(f, "registry"),
            Self::Decider => write!(f, "decider"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemServicesConfig {
    #[serde(default = "default_system_services")]
    pub enable: Vec<ServiceKey>,
    #[serde(default)]
    pub aqua_ipfs: AquaIpfsConfig,
    #[serde(default)]
    pub decider: DeciderConfig,
    #[serde(default)]
    pub registry: RegistryConfig,
    #[serde(default)]
    pub connector: ConnectorConfig,
}

impl Default for SystemServicesConfig {
    fn default() -> Self {
        Self {
            enable: default_system_services(),
            aqua_ipfs: Default::default(),
            decider: Default::default(),
            registry: Default::default(),
            connector: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AquaIpfsConfig {
    #[serde(default = "default_ipfs_multiaddr")]
    pub external_api_multiaddr: String,
    #[serde(default = "default_ipfs_multiaddr")]
    pub local_api_multiaddr: String,
    #[serde(default = "default_ipfs_binary_path")]
    pub ipfs_binary_path: String,
}

impl Default for AquaIpfsConfig {
    fn default() -> Self {
        Self {
            external_api_multiaddr: default_ipfs_multiaddr(),
            local_api_multiaddr: default_ipfs_multiaddr(),
            ipfs_binary_path: default_ipfs_binary_path(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConnectorConfig {
    #[serde(default = "default_curl_binary_path")]
    pub curl_binary_path: String,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            curl_binary_path: default_curl_binary_path(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeciderConfig {
    #[serde(default = "default_decider_spell_period_sec")]
    pub decider_period_sec: u32,
    #[serde(default = "default_worker_spell_period_sec")]
    pub worker_period_sec: u32,
    #[serde(default = "default_ipfs_multiaddr")]
    pub worker_ipfs_multiaddr: String,
    #[serde(default = "default_decider_network_api_endpoint")]
    pub network_api_endpoint: String,
    #[serde(default = "default_decider_network_id")]
    pub network_id: u64,
    #[serde(default = "default_matcher_address")]
    pub matcher_address: String,
    #[serde(default = "default_decider_start_block_hex")]
    pub start_block: String,
    #[serde(default = "default_decider_worker_gas")]
    pub worker_gas: u64,
    #[serde(default)]
    pub wallet_key: Option<String>,
}

impl Default for DeciderConfig {
    fn default() -> Self {
        Self {
            decider_period_sec: default_decider_spell_period_sec(),
            worker_period_sec: default_worker_spell_period_sec(),
            worker_ipfs_multiaddr: default_ipfs_multiaddr(),
            network_api_endpoint: default_decider_network_api_endpoint(),
            network_id: default_decider_network_id(),
            matcher_address: default_matcher_address(),
            start_block: default_decider_start_block_hex(),
            worker_gas: default_decider_worker_gas(),
            wallet_key: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RegistryConfig {
    #[serde(default = "default_registry_spell_period_sec")]
    pub registry_period_sec: u32,
    #[serde(default = "default_registry_expired_spell_period_sec")]
    pub expired_period_sec: u32,
    #[serde(default = "default_registry_renew_spell_period_sec")]
    pub renew_period_sec: u32,
    #[serde(default = "default_registry_replicate_spell_period_sec")]
    pub replicate_period_sec: u32,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            registry_period_sec: default_registry_spell_period_sec(),
            expired_period_sec: default_registry_expired_spell_period_sec(),
            renew_period_sec: default_registry_renew_spell_period_sec(),
            replicate_period_sec: default_registry_replicate_spell_period_sec(),
        }
    }
}
