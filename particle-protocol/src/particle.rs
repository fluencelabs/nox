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

use fluence_libp2p::{peerid_serializer, RandomPeerId};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Particle {
    pub id: String,
    #[serde(with = "peerid_serializer")]
    pub init_peer_id: PeerId,
    pub timestamp: u64,
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
    /// Must be an array
    pub data: serde_json::Value,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            // TODO: sure random peer id is OK as default?
            init_peer_id: RandomPeerId::random(),
            timestamp: 0,
            ttl: 0,
            script: "".to_string(),
            signature: vec![],
            data: json!([]),
        }
    }
}
