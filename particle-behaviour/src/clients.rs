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
/// Describes how peer is connected to this node
pub(super) enum ConnectionKind {
    /// Peer is connected to this node directly
    Direct,
    /// Peer isn't connected to this node
    Disconnected,
}

impl ParticleBehaviour {
    pub(super) fn add_peer(&mut self, peer: PeerId, address: Multiaddr) {
        self.connected_peers.insert(peer.clone(), address);
    }

    pub(super) fn peer_address(&self, peer: &PeerId) -> Option<&Multiaddr> {
        self.connected_peers.get(peer)
    }

    pub(super) fn remove_peer(&mut self, peer: &PeerId) {
        self.connected_peers.remove(peer);
    }

    /// Returns whether peer is a directly connected client or not
    pub(super) fn connection_kind(&self, peer: &PeerId) -> ConnectionKind {
        if let Some(_) = self.connected_peers.get(peer) {
            ConnectionKind::Direct
        } else {
            ConnectionKind::Disconnected
        }
    }
}
