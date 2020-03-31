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

use janus_server::misc::{multihash_serializer, peerid_serializer, provider_serializer};

use libp2p::PeerId;
use multihash::Multihash;
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};

/// Describes network messages from a peer to current node (client -> server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToNodeNetworkMsg {
    /// Represents a message that should be relayed to given dst node.
    Relay {
        #[serde(with = "peerid_serializer")]
        dst_id: PeerId,
        data: Vec<u8>,
    },
    Provide {
        #[serde(with = "multihash_serializer")]
        key: Multihash,
    },
    FindProviders {
        /// PeerId of the client who requested providers
        #[serde(with = "peerid_serializer")]
        client_id: PeerId,
        /// Key to find providers for
        #[serde(with = "multihash_serializer")]
        key: Multihash,
    },
}

/// Describes network message from current node to a peer (server -> client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ToPeerNetworkMsg {
    /// Message that should be relayed from src node to chosen dst node.
    Deliver {
        #[serde(with = "peerid_serializer")]
        src_id: PeerId,
        data: Vec<u8>,
    },
    /// Found providers for a given peer id
    Providers {
        /// PeerId of the client who requested providers
        #[serde(with = "peerid_serializer")]
        client_id: PeerId,
        /// Key to find providers for
        #[serde(with = "multihash_serializer")]
        key: Multihash,
        #[serde(with = "provider_serializer")]
        providers: Vec<(Multiaddr, PeerId)>,
    },
    // TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
    Upgrade,
}

// TODO: remove that. It's necessary for `Default` implementation, which seems semi-required by libp2p
impl Default for ToPeerNetworkMsg {
    fn default() -> Self {
        ToPeerNetworkMsg::Upgrade
    }
}
