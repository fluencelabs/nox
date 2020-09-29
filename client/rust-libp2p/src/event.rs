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
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use particle_protocol::Particle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientEvent {
    Particle {
        particle: Particle,
        #[serde(with = "peerid_serializer")]
        sender: PeerId,
    },
    NewConnection {
        #[serde(with = "peerid_serializer")]
        peer_id: PeerId,
        multiaddr: Multiaddr,
    },
}
