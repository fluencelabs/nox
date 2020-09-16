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

use crate::dht::{DHTEvent, SwarmEventType};
use crate::ParticleDHT;

use fluence_libp2p::poll_loop;

use libp2p::{
    core::{
        connection::{ConnectionId, ListenerId},
        ConnectedPoint, Multiaddr,
    },
    futures::task::{Context, Poll},
    kad::{store::MemoryStore, Kademlia},
    swarm::{IntoProtocolsHandler, NetworkBehaviour, PollParameters, ProtocolsHandler},
    PeerId,
};
use std::error::Error;

impl NetworkBehaviour for ParticleDHT {
    type ProtocolsHandler = <Kademlia<MemoryStore> as NetworkBehaviour>::ProtocolsHandler;
    type OutEvent = DHTEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.kademlia.new_handler()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        log::info!("addresses_of_peer {}", peer_id);
        self.kademlia.addresses_of_peer(peer_id)
    }

    fn inject_connected(&mut self, id: &PeerId) {
        log::debug!("{} got inject_connected {}", self.config.peer_id, id);
        self.kademlia.inject_connected(id);
    }

    fn inject_disconnected(&mut self, id: &PeerId) {
        log::debug!("{} got inject_disconnected {}", self.config.peer_id, id);
        self.kademlia.inject_disconnected(id);
    }

    fn inject_connection_established(&mut self, p: &PeerId, i: &ConnectionId, c: &ConnectedPoint) {
        #[rustfmt::skip]
        log::debug!("{} got connection_established {} {:?} {:?}", self.config.peer_id, p, i, c);
        self.kademlia.inject_connection_established(p, i, c);
    }

    fn inject_connection_closed(&mut self, p: &PeerId, i: &ConnectionId, c: &ConnectedPoint) {
        #[rustfmt::skip]
        log::debug!("{} got connection_closed {} {:?} {:?}", self.config.peer_id, p, i, c);
        self.kademlia.inject_connection_closed(p, i, c)
    }

    fn inject_event(
        &mut self,
        source: PeerId,
        connection_id: ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        log::debug!("{} got Kademlia event: {:?}", self.config.peer_id, event);
        self.kademlia.inject_event(source, connection_id, event);
    }

    fn inject_addr_reach_failure(&mut self, p: Option<&PeerId>, a: &Multiaddr, e: &dyn Error) {
        self.kademlia.inject_addr_reach_failure(p, a, e);
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        // TODO: clear connected_peers on inject_listener_closed?
        self.kademlia.inject_dial_failure(peer_id);
    }

    fn inject_new_listen_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_new_listen_addr(a)
    }

    fn inject_expired_listen_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_expired_listen_addr(a)
    }

    fn inject_new_external_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_new_external_addr(a)
    }

    fn inject_listener_error(&mut self, i: ListenerId, e: &(dyn Error + 'static)) {
        self.kademlia.inject_listener_error(i, e)
    }

    fn inject_listener_closed(&mut self, i: ListenerId, reason: Result<(), &std::io::Error>) {
        self.kademlia.inject_listener_closed(i, reason)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        poll_loop!(self, self.kademlia, cx, params);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
