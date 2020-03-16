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

use crate::node_service::relay::kademlia::Promise;
use crate::node_service::relay::{KademliaRelay, Provider};
use libp2p::kad::record::Key;
use libp2p::PeerId;
use parity_multihash::Multihash;

impl Provider for KademliaRelay {
    fn provide(&mut self, key: Multihash) {
        let key: Key = key.into();

        self.kademlia.start_providing(key);
    }

    fn find_providers(&mut self, client_id: PeerId, key: Multihash) {
        self.enqueue(
            key.clone(),
            Promise::FindProviders {
                client_id,
                key: key.clone(),
            },
        );

        self.get_providers(key);
    }
}
