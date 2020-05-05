/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use super::router::WaitPeer;
use super::FunctionRouter;
use faas_api::FunctionCall;
use janus_libp2p::SafeMultihash;
use libp2p::{
    kad::{GetClosestPeersError, GetClosestPeersOk, GetClosestPeersResult},
    swarm::{DialPeerCondition, NetworkBehaviour, NetworkBehaviourAction},
    PeerId,
};

// Contains methods related to searching for and connecting to peers
impl FunctionRouter {
    // Look for peer in Kademlia, enqueue call to wait for result
    pub(super) fn search_for_peer(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.wait_peer
            .enqueue(peer_id.clone(), WaitPeer::Found(call));
        // TODO: don't call get_closest_peers if there are already some calls waiting for it
        self.kademlia
            .get_closest_peers(SafeMultihash::from(peer_id))
    }

    // Take all calls waiting for this peer to be found, and send them
    pub(super) fn found_closest(&mut self, result: GetClosestPeersResult) {
        let (key, peers) = match result {
            Ok(GetClosestPeersOk { key, peers }) => (key, peers),
            Err(GetClosestPeersError::Timeout { key, peers }) => {
                let will_try = if !peers.is_empty() {
                    " Will try to send calls anyway"
                } else {
                    ""
                };
                log::error!(
                    "Timed out while finding closest peer {}. Found only {} peers.{}",
                    bs58::encode(&key).into_string(),
                    peers.len(),
                    will_try
                );
                (key, peers)
            }
        };

        let peer_id = match PeerId::from_bytes(key) {
            Err(err) => {
                log::error!(
                    "Can't parse peer id from GetClosestPeersResult key: {}",
                    bs58::encode(err).into_string()
                );
                return;
            }
            Ok(peer_id) => peer_id,
        };

        let calls = self
            .wait_peer
            .remove_with(&peer_id, |wp| matches!(wp, WaitPeer::Found(_)));
        // TODO: is `peers.contains` necessary? Not sure
        // Check if peer is in the routing table.
        let routable = peers.contains(&peer_id) || self.is_routable(&peer_id);
        if routable {
            for call in calls {
                // Send the message once peer is connected
                self.send_to_routable(peer_id.clone(), call.call())
            }
        } else {
            for call in calls {
                // Report error
                self.send_error_on_call(
                    call.call(),
                    "Peer wasn't found via GetClosestPeers".to_string(),
                )
            }
        }
    }

    pub(super) fn connect(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.wait_peer
            .enqueue(peer_id.clone(), WaitPeer::Connected(call));

        log::info!("Dialing {}", peer_id.to_base58());
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id,
            condition: DialPeerCondition::Disconnected,
        });
    }

    pub(super) fn connected(&mut self, peer_id: PeerId) {
        log::info!("Peer connected: {}", peer_id.to_base58());
        self.connected_peers.insert(peer_id.clone());

        let waiting = self
            .wait_peer
            .remove_with(&peer_id, |wp| matches!(wp, WaitPeer::Connected(_)));

        // TODO: leave move or remove move from closure?
        waiting.for_each(move |wp| match wp {
            WaitPeer::Connected(call) => self.send_to_connected(peer_id.clone(), call),
            WaitPeer::Found(_) => unreachable!("Can't happen. Just filtered WaitPeer::Connected"),
        });
    }

    // TODO: clear connected_peers on inject_listener_closed?
    pub(super) fn disconnected(&mut self, peer_id: &PeerId) {
        log::info!(
            "Peer disconnected: {}. {} calls left waiting.",
            peer_id.to_base58(),
            self.wait_peer.count(&peer_id)
        );
        self.connected_peers.remove(peer_id);

        self.remove_delegated_services(peer_id);

        let waiting_calls = self.wait_peer.remove(peer_id);
        for waiting in waiting_calls.into_iter() {
            self.send_error_on_call(waiting.call(), "Peer disconnected".into());
        }
    }

    pub(super) fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains(peer_id)
    }

    // Whether peer is in the routing table
    pub(super) fn is_routable(&mut self, peer_id: &PeerId) -> bool {
        // TODO: Checking `is_connected` inside `is_routable` smells...
        let connected = self.is_connected(peer_id);

        let kad = self.kademlia.addresses_of_peer(peer_id);
        let in_kad = !kad.is_empty();

        log::debug!(
            "peer {} in routing table: Connected? {} Kademlia {:?}",
            peer_id.to_base58(),
            connected,
            kad
        );
        connected || in_kad
    }

    // Whether given peer id is equal to ours
    pub(super) fn is_local(&self, peer_id: &PeerId) -> bool {
        let me = self.peer_id.to_base58();
        let they = peer_id.to_base58();
        if me == they {
            log::debug!("{} is ME! (equals to {})", they, me);
            true
        } else {
            log::debug!("{} is THEM! (not equal to {})", they, me);
            false
        }
    }
}
