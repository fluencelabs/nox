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
use faas_api::{FunctionCall, Protocol};
use libp2p::{
    swarm::{DialPeerCondition, NetworkBehaviour, NetworkBehaviourAction},
    PeerId,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PeerStatus {
    Connected = 3000, // Connected is better than Routable
    Routable = 2000,  // Router is better than None
    Unknown = 1000,   // Unknown is bottom
}

// Contains methods related to searching for and connecting to peers
impl FunctionRouter {
    /// Look for peer in Kademlia, enqueue call to wait for result
    pub(super) fn search_then_send(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.query_closest(peer_id, WaitPeer::Routable(call))
    }

    pub(super) fn send_to_neighborhood(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.query_closest(peer_id, WaitPeer::Neighborhood(call))
    }

    /// Query for peers closest to the `peer_id` as DHT key, enqueue call until response
    fn query_closest(&mut self, peer_id: PeerId, call: WaitPeer) {
        self.wait_peer.enqueue(peer_id.clone(), call);
        // TODO: don't call get_closest_peers if there are already some calls waiting for it
        self.kademlia.get_closest_peers(peer_id)
    }

    /// Send all calls waiting for this peer to be found
    pub(super) fn found_closest(&mut self, key: Vec<u8>, peers: Vec<PeerId>) {
        use PeerStatus as ExpectedStatus;

        let peer_id = match PeerId::from_bytes(key) {
            Err(err) => {
                log::warn!(
                    "Found closest peers for invalid key {}: not a PeerId",
                    bs58::encode(err).into_string()
                );
                return;
            }
            Ok(peer_id) => peer_id,
        };

        // Forward to `peer_id`
        let calls = self.wait_peer.remove_with(&peer_id, |wp| wp.found());
        for call in calls {
            if peers.is_empty() {
                // No peers found, send error
                self.send_error_on_call(call.into(), "peer wasn't found via closest query".into())
            } else {
                // Forward calls to `peer_id`, assuming it is now routable
                self.send_to(peer_id.clone(), ExpectedStatus::Routable, call.into())
            }
        }

        // Forward to neighborhood
        let calls = self.wait_peer.remove_with(&peer_id, |wp| wp.neighborhood());
        for call in calls {
            let call: FunctionCall = call.into();

            // Check if any peers found, if not – send error
            if peers.is_empty() {
                self.send_error_on_call(call.into(), "neighborhood was empty".into());
                return;
            }

            // Forward calls to each peer in neighborhood
            for peer_id in peers.iter() {
                let mut call = call.clone();
                // Modify target: prepend peer
                let target =
                    Protocol::Peer(peer_id.clone()) / call.target.take().unwrap_or_default();
                self.send_to(
                    peer_id.clone(),
                    ExpectedStatus::Routable,
                    call.with_target(target),
                )
            }
        }
    }

    pub(super) fn connect_then_send(&mut self, peer_id: PeerId, call: FunctionCall) {
        use DialPeerCondition::Disconnected as condition; // o_O you can do that?!
        use NetworkBehaviourAction::DialPeer;
        use WaitPeer::Connected;

        self.wait_peer.enqueue(peer_id.clone(), Connected(call));

        log::info!("Dialing {}", peer_id);
        self.events.push_back(DialPeer { peer_id, condition });
    }

    pub(super) fn connected(&mut self, peer_id: PeerId) {
        log::info!("Peer connected: {}", peer_id);
        self.connected_peers.insert(peer_id.clone());

        let waiting = self.wait_peer.remove_with(&peer_id, |wp| wp.connected());

        // TODO: leave move or remove move from closure?
        waiting.for_each(move |wp| match wp {
            WaitPeer::Connected(call) => self.send_to_connected(peer_id.clone(), call),
            _ => unreachable!("Can't happen. Just filtered WaitPeer::Connected"),
        });
    }

    // TODO: clear connected_peers on inject_listener_closed?
    pub(super) fn disconnected(&mut self, peer_id: &PeerId) {
        log::info!(
            "Peer disconnected: {}. {} calls left waiting.",
            peer_id,
            self.wait_peer.count(&peer_id)
        );
        self.connected_peers.remove(peer_id);

        self.remove_halted_names(peer_id);

        let waiting_calls = self.wait_peer.remove(peer_id);
        for waiting in waiting_calls.into_iter() {
            self.send_error_on_call(waiting.into(), format!("peer {} disconnected", peer_id));
        }
    }

    pub(super) fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains(peer_id)
    }

    /// Whether peer is in the routing table
    pub(super) fn is_routable(&mut self, peer_id: &PeerId) -> bool {
        // TODO: Checking `is_connected` inside `is_routable` smells...
        let connected = self.is_connected(peer_id);

        let kad = self.kademlia.addresses_of_peer(peer_id);
        let in_kad = !kad.is_empty();

        log::debug!(
            "peer {} in routing table: Connected? {} Kademlia {:?}",
            peer_id,
            connected,
            kad
        );
        connected || in_kad
    }

    /// Whether given peer id is equal to ours
    pub(super) fn is_local(&self, peer_id: &PeerId) -> bool {
        if self.peer_id.eq(peer_id) {
            log::debug!("{} is LOCAL", peer_id);
            true
        } else {
            log::debug!("{} is REMOTE", peer_id);
            false
        }
    }

    pub(super) fn search_for_client(&mut self, client: PeerId, _call: FunctionCall) {
        self.resolve_name(Protocol::Client(client).into())
    }

    pub(super) fn peer_status(&mut self, peer: &PeerId) -> PeerStatus {
        use PeerStatus::*;

        if self.is_connected(peer) {
            Connected
        } else if self.is_routable(peer) {
            Routable
        } else {
            Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_status() {
        use PeerStatus::*;

        assert!(Connected > Unknown);
        assert!(Connected > Routable);
        assert_eq!(Connected, Connected);

        assert!(Routable > Unknown);
        assert!(Routable < Connected);
        assert_eq!(Routable, Routable);

        assert!(Unknown < Connected);
        assert!(Unknown < Routable);
        assert_eq!(Unknown, Unknown);
    }
}
