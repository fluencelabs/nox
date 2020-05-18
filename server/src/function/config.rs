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

use faas_api::{Address, Protocol};
use libp2p::{identity::ed25519::Keypair, PeerId};
use parity_multiaddr::Multiaddr;

/// Router static info: local peer id, keypair and external addresses
pub struct RouterConfig {
    /// Keypair, currently used to sign DHT records
    pub(super) keypair: Keypair,
    // TODO: store peer_id as Lazy::new(|| kp.to_peer_id())?
    /// Peer id of this node
    pub(super) peer_id: PeerId,
    /// Addresses this node is reachable on, used in Identify builtin service
    pub(super) external_addresses: Vec<Multiaddr>,
}

impl RouterConfig {
    pub(crate) fn new(
        keypair: Keypair,
        peer_id: PeerId,
        external_addresses: Vec<Multiaddr>,
    ) -> Self {
        Self {
            keypair,
            peer_id,
            external_addresses,
        }
    }

    /// Create `/peer/QmPeer` address for local peer id
    pub(super) fn local_address(&self) -> Address {
        Protocol::Peer(self.peer_id.clone()).into()
    }
}
