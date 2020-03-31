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

use crate::misc::peerid_serializer;
use crate::peer_service::messages::ToPeerMsg;

use libp2p::PeerId;
use serde::{Deserialize, Serialize};

/// Relay event is just a data that need to be relayed from a peer of `src_id` to a peer of `dst_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayMessage {
    #[serde(with = "peerid_serializer")]
    pub src_id: PeerId,
    #[serde(with = "peerid_serializer")]
    pub dst_id: PeerId,
    pub data: Vec<u8>,
}

impl Into<ToPeerMsg> for RelayMessage {
    fn into(self) -> ToPeerMsg {
        ToPeerMsg::Deliver {
            src_id: self.src_id,
            dst_id: self.dst_id,
            data: self.data,
        }
    }
}

// this needed only to implement default for libp2p handler
// and shouldn't be used for other purposes
impl Default for RelayMessage {
    fn default() -> Self {
        RelayMessage {
            src_id: PeerId::random(),
            dst_id: PeerId::random(),
            data: Vec::new(),
        }
    }
}
