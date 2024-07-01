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
