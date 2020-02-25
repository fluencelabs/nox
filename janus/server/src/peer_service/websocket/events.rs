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
pub enum WebsocketEvent {
    /// Represents an event that should be relayed to a given destination node
    /// or an event that already relayed and will be sent to a connected client.
    Relay {
        peer_id: String,
        data: String,
        /// the public key of the message's author
        /// todo should be not in a relay event, but in some authorization process
        p_key: String,
        signature: String,
    },

    /// Send message when error occurred
    Error {
        err_msg: String,
    },
    // TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
    Upgrade,
}

// TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
impl Default for WebsocketEvent {
    fn default() -> Self {
        WebsocketEvent::Upgrade
    }
}
