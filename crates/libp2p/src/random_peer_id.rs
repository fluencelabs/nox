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

use libp2p::identity::Keypair;
use libp2p::PeerId;

pub struct RandomPeerId();
impl RandomPeerId {
    /// Generates PeerId from random Ed25519 key
    pub fn random() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }
}
