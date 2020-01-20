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

/// Describes network messages from a node to current peer (client -> server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WebsocketMessage {
    /// Represents a message that should be relayed to given dst node.
    Relay { peer_id: String, data: String },

    /// Requests for the network state.
    /// Currently, gives the whole peers in the network, this behaviour will be refactored in future.
    GetNetworkState,
    NetworkState { peers: Vec<String> }
}

impl Default for WebsocketMessage {
    fn default() -> Self {
        WebsocketMessage::GetNetworkState
    }
}
