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

use fluence_libp2p::types::{OneshotOutlet, Outlet};
use particle_protocol::Particle;

use futures::future::BoxFuture;
use libp2p::core::Multiaddr;
use libp2p::PeerId;

#[derive(Debug, Clone)]
pub struct Contact {
    pub peer_id: PeerId,
    pub addr: Option<Multiaddr>,
}

pub trait ConnectionPool {
    fn connect(&mut self, contact: Contact) -> BoxFuture<bool>;
    fn disconnect(&mut self, contact: Contact) -> BoxFuture<bool>;
    fn is_connected(&self, peer_id: &PeerId) -> bool;
    fn get_contact(&self, peer_id: &PeerId) -> Option<Contact>;

    fn send(&mut self, to: Contact, particle: Particle) -> BoxFuture<bool>;
}

/*
// aqua-accessible (for `is_connected` builtin + xor)
is_connected: PubKey -> Bool

// Contact algebra
get_contact: PubKey -> Contact?
is_connected: Contact -> Bool
connect: Contact -> async Bool // Bool or Either
disconnect: Contact -> async Bool // Bool or Either

// number of active (existing) connections
count_connections: {clients: Int, peers: Int}
//conn lifecycle
lifecycle_events: (  Connected(Contact) | Disconnected(Contact)  )*
// receive msg (particle)
source: (Contact, msg)*
// send msg (particle)
sink: (Contact, msg) -> async Bool // Bool or Either
*/
