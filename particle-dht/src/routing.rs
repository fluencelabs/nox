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

use crate::ParticleDHT;
use libp2p::swarm::NetworkBehaviour;
use libp2p::PeerId;
use particle_protocol::Particle;

#[derive(Debug)]
pub(super) enum PeerStatus {
    /// Peer is expected to be connected
    Connected,
    /// Peer is expected to be in the routing table
    Routable,
    /// State of the peer is unknown, need to look it up in kademlia
    Unknown,
}

impl ParticleDHT {
    pub fn send_to(&mut self, to: PeerId, particle: Particle) {
        use PeerStatus::*;

        let status = self.peer_status(&to);

        #[rustfmt::skip]
        log::debug!("Sending particle {} to peer {} ({:?})", particle.id, to, status);

        match status {
            Connected => self.send_to_connected(to, particle),
            Routable => self.connect_then_send(to, particle),
            Unknown => self.search_then_send(to, particle),
        }
    }

    pub(super) fn peer_status(&mut self, peer: &PeerId) -> PeerStatus {
        use PeerStatus::*;

        if self.is_connected(peer) {
            return Connected;
        }
        if self.is_routable(peer) {
            return Routable;
        }
        Unknown
    }

    /// Whether peer is in the routing table
    pub(super) fn is_routable(&mut self, peer_id: &PeerId) -> bool {
        let in_kad = !self.kademlia.addresses_of_peer(peer_id).is_empty();
        log::debug!("peer {} in routing table? {:?}", peer_id, in_kad);
        in_kad
    }

    /// Whether given peer id is equal to ours
    pub(super) fn is_local(&self, peer_id: &PeerId) -> bool {
        self.config.peer_id.eq(peer_id)
    }
}
