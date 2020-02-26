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

use serde::{Deserialize, Serialize};

/// Describes network messages from a peer to current node (client -> server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToNodeEvent {
    /// Represents a message that should be relayed to given dst node.
    Relay { dst_id: Vec<u8>, data: Vec<u8> },
}

/// Describes network message from current node to a peer (server -> client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToPeerEvent {
    /// Message that should be relayed from src node to chosen dst node.
    Deliver {
        src_id: Vec<u8>,
        data: Vec<u8>,
    },
    // TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
    Upgrade,
}

// TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
impl Default for ToPeerEvent {
    fn default() -> Self {
        ToPeerEvent::Upgrade
    }
}
