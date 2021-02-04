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
use json_utils::base64_serde;

use derivative::Derivative;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize, PartialEq, Derivative)]
#[derivative(Debug)]
pub struct Particle {
    pub id: String,
    #[serde(with = "peerid_serializer")]
    pub init_peer_id: PeerId,
    pub timestamp: u64,
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
    /// base64-encoded
    #[serde(with = "base64_serde")]
    #[derivative(Debug(format_with = "fmt_data"))]
    pub data: Vec<u8>,
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
            data: vec![],
        }
    }
}

impl Particle {
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = self.timestamp.checked_add(self.ttl as u64) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before Unix epoch")
                .as_millis();

            return deadline as u128 > now;
        }

        // If timestamp + ttl overflows u64, consider particle expired
        true
    }
}

#[allow(clippy::ptr_arg)]
fn fmt_data(data: &Vec<u8>, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    write!(f, "{}", bs58::encode(data).into_string())
}
