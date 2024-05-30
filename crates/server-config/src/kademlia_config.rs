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

use libp2p::StreamProtocol;
use std::time::Duration;

use crate::Network;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// see `libp2p_kad::KademliaConfig`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnresolvedKademliaConfig {
    pub max_packet_size: Option<usize>,
    #[serde(with = "humantime_serde")]
    pub query_timeout: Duration,
    pub replication_factor: Option<usize>,
    /// Number of times peer is failed to be discovered before it is banned
    pub peer_fail_threshold: usize,
    /// Period after which peer ban is lifted
    #[serde(with = "humantime_serde")]
    pub ban_cooldown: Duration,
}

impl UnresolvedKademliaConfig {
    pub fn resolve(&self, network: &Network) -> eyre::Result<KademliaConfig> {
        let protocol_name: StreamProtocol = network.try_into()?;

        Ok(KademliaConfig {
            max_packet_size: self.max_packet_size,
            query_timeout: self.query_timeout,
            replication_factor: self.replication_factor,
            peer_fail_threshold: self.peer_fail_threshold,
            ban_cooldown: self.ban_cooldown,
            protocol_name,
        })
    }
}

/// see `libp2p_kad::KademliaConfig`
#[serde_as]
#[derive(Debug, Clone, Serialize)]
pub struct KademliaConfig {
    pub max_packet_size: Option<usize>,
    #[serde(with = "humantime_serde")]
    pub query_timeout: Duration,
    pub replication_factor: Option<usize>,
    /// Number of times peer is failed to be discovered before it is banned
    pub peer_fail_threshold: usize,
    /// Period after which peer ban is lifted
    #[serde(with = "humantime_serde")]
    pub ban_cooldown: Duration,
    #[serde_as(as = "DisplayFromStr")]
    pub protocol_name: StreamProtocol,
}

impl Default for UnresolvedKademliaConfig {
    fn default() -> Self {
        Self {
            max_packet_size: Some(100 * 4096 * 4096), // 100Mb
            query_timeout: Duration::from_secs(3),
            replication_factor: None,
            peer_fail_threshold: 3,
            ban_cooldown: Duration::from_secs(60),
        }
    }
}
