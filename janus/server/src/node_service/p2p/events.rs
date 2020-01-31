/*
 * Copyright 2019 Fluence Labs Limited
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

/// This message type is intended to describe network topology changing
/// (adding or removing new nodes or peers).
#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum P2PNetworkEvents {
    // TODO: add implementation of serde serialize/deserialize for PeerId
    NodeConnected {
        node_id: Vec<u8>,
        peer_ids: Vec<Vec<u8>>,
    },
    PeersConnected {
        node_id: Vec<u8>,
        peer_ids: Vec<Vec<u8>>,
    },
    PeersDisconnected {
        node_id: Vec<u8>,
        peer_ids: Vec<Vec<u8>>,
    },
    NodeDisconnected {
        node_id: Vec<u8>,
    },

    /// Send by bootstrap nodes on the first connection.
    NetworkState {
        // list of node PeerId, node multiaddrs, peers PeerIds
        network_map: Vec<(Vec<u8>, Vec<String>, Vec<Vec<u8>>)>,
    },
}
