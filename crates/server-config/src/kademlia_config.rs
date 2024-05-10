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

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// see `libp2p_kad::KademliaConfig`
#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

impl Default for KademliaConfig {
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

