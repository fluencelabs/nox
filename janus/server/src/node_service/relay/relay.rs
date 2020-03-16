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

use crate::node_service::relay::RelayMessage;

use libp2p::PeerId;
use parity_multiaddr::Multiaddr;

pub trait Relay {
    /// New network address for a connected node is found, add it
    fn add_node_addresses(&mut self, node_id: &PeerId, addresses: Vec<Multiaddr>);

    /// New peer is connected locally, store it
    fn add_local_peer(&mut self, peer_id: PeerId);

    /// Locally connected peer has disconnected
    fn remove_local_peer(&mut self, peer_id: &PeerId);

    /// Instructs node to bootstrap itself: walk through Kademlia or gossip NodeConnected, whatever
    fn bootstrap(&mut self);

    /// Relays event to specified destination
    fn relay(&mut self, event: RelayMessage);
}
