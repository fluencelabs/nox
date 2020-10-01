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

use crate::wait_peer::WaitPeer;
use crate::ParticleDHT;
use libp2p::kad::record::Key;
use libp2p::PeerId;
use particle_protocol::Particle;

impl ParticleDHT {
    pub fn resolve(&mut self, _key: Key) {
        unimplemented!("TODO")
    }

    /// Look for peer in Kademlia, enqueue call to wait for result
    pub(super) fn search_then_send(&mut self, peer_id: PeerId, particle: Particle) {
        self.query_closest(peer_id, WaitPeer::Routable(particle))
    }

    /// Query for peers closest to the `peer_id` as DHT key, enqueue call until response
    fn query_closest(&mut self, peer_id: PeerId, call: WaitPeer) {
        log::debug!("Queried closest peers for peer_id {}", peer_id);
        // TODO: Don't call get_closest_peers if there are already WaitPeer::Neighbourhood or WaitPeer::Routable enqueued
        self.wait_peer.enqueue(peer_id.clone(), call);
        // NOTE: Using Qm form of `peer_id` here (via peer_id.borrow), since kademlia uses that for keys
        self.kademlia.get_closest_peers(peer_id);
    }

    pub(super) fn found_closest(&mut self, peer_id: PeerId, peers: Vec<PeerId>) {
        log::debug!("Found closest peers for peer_id {}: {:?}", peer_id, peers);
        // Forward to `peer_id`
        let particles = self
            .wait_peer
            .remove_with(peer_id.clone(), |wp| wp.routable());

        if peers.is_empty() || !peers.iter().any(|p| p == &peer_id) {
            for _particle in particles {
                let err_msg = format!(
                    "peer {} wasn't found via closest query: {:?}",
                    peer_id,
                    peers.iter().map(|p| p.to_base58())
                );
                log::warn!("{}", err_msg);
                // Peer wasn't found, send error
                // self.send_error_on_call(particle.into(), err_msg)
                unimplemented!("error handling")
            }
        } else {
            for particle in particles {
                // Forward calls to `peer_id`, guaranteeing it is now routable
                self.connect_then_send(peer_id.clone(), particle.into())
            }
        }
    }
}
