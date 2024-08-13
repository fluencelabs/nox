/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use super::defaults::*;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::path::PathBuf;

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
        match name {
            "aqua-ipfs" => Some(ServiceKey::AquaIpfs),
            "trust-graph" => Some(ServiceKey::TrustGraph),
            "registry" => Some(ServiceKey::Registry),
            "decider" => Some(ServiceKey::Decider),
            _ => None,
        }
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
    pub ipfs_binary_path: PathBuf,
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
    pub curl_binary_path: PathBuf,
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
}

impl Default for DeciderConfig {
    fn default() -> Self {
        Self {
            decider_period_sec: default_decider_spell_period_sec(),
            worker_period_sec: default_worker_spell_period_sec(),
            worker_ipfs_multiaddr: default_ipfs_multiaddr(),
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
