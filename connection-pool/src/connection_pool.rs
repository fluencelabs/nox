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

use fluence_libp2p::types::Outlet;
use particle_protocol::Particle;

use libp2p::core::Multiaddr;
use libp2p::PeerId;

struct Contact {
    peer_id: PeerId,
    addr: Option<Multiaddr>,
}

trait ConnectionPool {
    // TODO: return error? How, when?
    //      for example if contact isn't reachable, but is that possible?
    //      for client – NO. At first seems like yes, but if a client isn't connected, how do we know it's a client?
    //      for peer – not sure.
    fn send(to: Contact, particle: Particle);

    // TODO: how to implement it?
    //       for clients – seems easy, but we only know it is a client if it is connected
    //       for peers – how? Maybe only implement for clients, and for peers always return 'false'
    // TODO: maybe name it 'is_client_connected'
    fn is_connected(peer_id: PeerId) -> Outlet<bool>;
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
