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

use libp2p::PeerId;
use log::trace;
use parity_multiaddr::Multiaddr;

use crate::node_service::relay::KademliaRelay;
use crate::node_service::relay::Relay;
use crate::node_service::relay::RelayEvent;

impl Relay for KademliaRelay {
    fn add_node_addresses(&mut self, node_id: &PeerId, addresses: Vec<Multiaddr>) {
        trace!(
            "adding new node {} with {} addresses",
            node_id.to_base58(),
            addresses.len()
        );
        for addr in addresses {
            self.kademlia.add_address(node_id, addr);
        }
    }

    fn add_local_peer(&mut self, peer_id: PeerId) {
        self.announce_peer(peer_id)
    }

    fn remove_local_peer(&mut self, peer_id: &PeerId) {
        self.bury_peer(peer_id)
    }

    fn bootstrap(&mut self) {
        self.kademlia.bootstrap();
    }

    fn relay(&mut self, event: RelayEvent) {
        if let Ok(dst) = PeerId::from_bytes(event.dst_id.clone()) {
            trace!("relay event to {}", dst);
        }

        // Try to relay locally
        if !self.relay_local(event.clone()) {
            // If peer wasn't found locally, send remotely
            self.relay_remote(event);
        }
    }
}
