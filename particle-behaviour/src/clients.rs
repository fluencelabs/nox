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

use crate::ParticleBehaviour;
use libp2p::core::Multiaddr;
use libp2p::PeerId;

#[derive(Debug)]
pub(super) enum PeerKind {
    Client,
    Unknown,
}

impl ParticleBehaviour {
    pub(super) fn add_client_address(&mut self, client: PeerId, address: Multiaddr) {
        log::debug!("Got new peer address {} -> {:?}", client, address);

        if let Some(addr) = self.clients.insert(client.clone(), address) {
            log::debug!("Replaced old addr {} for client {}", addr, client)
        }
    }

    pub(super) fn client_address(&self, client: &PeerId) -> Option<&Multiaddr> {
        self.clients.get(client)
    }

    pub(super) fn remove_client(&mut self, client: &PeerId) {
        self.clients.remove(client);
    }

    /// Returns whether peer is a directly connected client or not
    pub(super) fn peer_kind(&self, peer: &PeerId) -> PeerKind {
        if let Some(_) = self.clients.get(peer) {
            PeerKind::Client
        } else {
            PeerKind::Unknown
        }
    }
}
