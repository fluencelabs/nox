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
use crate::{DHTEvent, ParticleDHT};
use libp2p::{swarm::DialPeerCondition, PeerId};

use particle_protocol::Particle;

impl ParticleDHT {
    pub(super) fn send_to_connected(&mut self, target: PeerId, particle: Particle) {
        self.emit(DHTEvent::Forward { target, particle })
    }

    pub(super) fn connect_then_send(&mut self, peer_id: PeerId, particle: Particle) {
        use super::wait_peer::WaitPeer::Connected;
        use DHTEvent::DialPeer;
        use DialPeerCondition::Disconnected as condition;

        self.wait_peer.enqueue(peer_id.clone(), Connected(particle));

        log::info!("Dialing {}", peer_id);
        self.push_event(DialPeer { peer_id, condition });
    }

    pub fn connected(&mut self, peer_id: PeerId) {
        log::info!("Peer connected: {}", peer_id);
        self.connected_peers.insert(peer_id.clone());

        let waiting = self
            .wait_peer
            .remove_with(peer_id.clone(), |wp| wp.connected());

        waiting.for_each(move |wp| match wp {
            WaitPeer::Connected(particle) => self.send_to_connected(peer_id.clone(), particle),
            _ => unreachable!("Can't happen. Just filtered WaitPeer::Connected"),
        });
    }

    // TODO: clear connected_peers on inject_listener_closed?
    // TODO: handle waiting call (wait_peer.count > 0)
    pub fn disconnected(&mut self, peer_id: &PeerId) {
        log::info!("Peer disconnected: {}{}", peer_id, {
            let count = self.wait_peer.count(&peer_id);
            if count > 0 {
                format!(" {} calls left waiting", count)
            } else {
                "".to_string()
            }
        });
        self.connected_peers.remove(peer_id);

        let _waiting_calls = self.wait_peer.remove(peer_id);
        /*for waiting in waiting_calls.into_iter() {
            self.send_error_on_call(waiting.into(), format!("peer {} disconnected", peer_id));
        }*/
    }

    pub(super) fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains(peer_id)
    }
}
