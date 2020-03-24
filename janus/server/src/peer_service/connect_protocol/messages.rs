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

use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};

pub type PeerIdBytes = Vec<u8>;
pub type MultihashBytes = Vec<u8>;

/// Describes network messages from a peer to current node (client -> server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToNodeNetworkMsg {
    /// Represents a message that should be relayed to given dst peer.
    Relay {
        dst_id: PeerIdBytes,
        data: PeerIdBytes,
    },
    Provide {
        key: MultihashBytes,
    },
    FindProviders {
        /// PeerId of the client who requested providers
        client_id: PeerIdBytes,
        /// Key to find providers for
        key: MultihashBytes,
    },
    // TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
    Upgrade,
}

// TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
impl Default for ToNodeNetworkMsg {
    fn default() -> Self {
        ToNodeNetworkMsg::Upgrade
    }
}

/// Describes network message from current node to a peer (server -> client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToPeerNetworkMsg {
    /// Message that should be relayed from src peer to chosen dst peer.
    Deliver {
        src_id: PeerIdBytes,
        data: PeerIdBytes,
    },
    Providers {
        /// PeerId of the client who requested providers
        client_id: PeerIdBytes,
        /// Key to find providers for
        key: MultihashBytes,
        providers: Vec<(Multiaddr, PeerIdBytes)>,
    },
}
