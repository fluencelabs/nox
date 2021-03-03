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

use fluence_libp2p::peerid_serializer;

use itertools::Itertools;
use libp2p::{core::Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Contact {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
}

impl Contact {
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self { peer_id, addresses }
    }
}

impl Display for Contact {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.addresses.is_empty() {
            write!(f, "{} @ [no addr]", self.peer_id)
        } else {
            let addrs = self.addresses.iter().join(" ");
            write!(f, "{} @ [{}]", self.peer_id, addrs)
        }
    }
}
