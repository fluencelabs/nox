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

use crate::behaviour::SwarmEventType;
use crate::ParticleBehaviour;

use particle_protocol::{HandlerMessage, ProtocolConfig};

use fluence_libp2p::{poll_loop, remote_multiaddr};

use libp2p::{
    core::{
        connection::{ConnectionId, ListenerId},
        either::EitherOutput,
        ConnectedPoint, Multiaddr,
    },
    kad::{store::MemoryStore, Kademlia},
    swarm::{
        IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour,
        NetworkBehaviourEventProcess, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use std::ops::DerefMut;
use std::{
    error::Error,
    task::{Context, Poll},
};

impl NetworkBehaviour for ParticleBehaviour {
    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage>,
        <Kademlia<MemoryStore> as NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(self.protocol_config.clone().into(), self.dht.new_handler())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        let p = self.peer_address(peer_id).cloned().into_iter();
        let d = self.dht.addresses_of_peer(peer_id).into_iter();

        p.chain(d).collect()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        self.dht.connected(peer_id.clone());
        self.dht.inject_connected(peer_id);

        // TODO: check that peer_id belongs to client (not node), and publish it
        //  self.dht.publish_client(peer_id.clone());
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        // TODO: self.dht.unpublish_client(peer_id)
        self.remove_peer(peer_id);
        self.dht.disconnected(peer_id);
        self.dht.inject_disconnected(peer_id);
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        use EitherOutput::{First, Second};

        match event {
            First(event) => match event {
                HandlerMessage::Particle(particle) => {
                    self.plumber.ingest(particle);
                    self.wake();
                }
                HandlerMessage::Upgrade => {}
                HandlerMessage::InboundUpgradeError(err) => log::warn!("UpgradeError: {:?}", err),
            },
            Second(event) => {
                NetworkBehaviour::inject_event(self.dht.deref_mut(), peer_id, connection, event)
            }
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        if let Poll::Ready(event) = self.plumber.poll(cx) {
            NetworkBehaviourEventProcess::inject_event(self, event);
        }

        if let Poll::Ready(event) = self.dht.poll(cx) {
            NetworkBehaviourEventProcess::inject_event(self, event);
        }

        if let Poll::Ready(cmd) = self.mailbox.poll(cx) {
            NetworkBehaviourEventProcess::inject_event(self, cmd);
        }

        // Poll kademlia and forward GenerateEvent to self.dht
        poll_loop!(
            &mut self.dht,
            self.dht.deref_mut(),
            cx,
            params,
            EitherOutput::Second
        );

        Poll::Pending
    }

    // ==== useless repetition below ====
    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        addr: &Multiaddr,
        error: &dyn Error,
    ) {
        self.dht.inject_addr_reach_failure(peer_id, addr, error);
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        self.dht.inject_dial_failure(peer_id);
    }

    fn inject_new_listen_addr(&mut self, addr: &Multiaddr) {
        self.dht.inject_new_listen_addr(addr);
    }

    fn inject_expired_listen_addr(&mut self, addr: &Multiaddr) {
        self.dht.inject_expired_listen_addr(addr);
    }

    fn inject_new_external_addr(&mut self, addr: &Multiaddr) {
        self.dht.inject_new_external_addr(addr);
    }

    fn inject_listener_error(&mut self, id: ListenerId, err: &(dyn std::error::Error + 'static)) {
        self.dht.inject_listener_error(id, err);
    }

    fn inject_listener_closed(&mut self, id: ListenerId, reason: Result<(), &std::io::Error>) {
        self.dht.inject_listener_closed(id, reason);
    }

    fn inject_connection_established(
        &mut self,
        id: &PeerId,
        ci: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let maddr = remote_multiaddr(cp);
        self.add_peer(id.clone(), maddr.clone());
        self.dht.inject_connection_established(id, ci, cp);
    }

    fn inject_connection_closed(&mut self, id: &PeerId, ci: &ConnectionId, cp: &ConnectedPoint) {
        self.dht.inject_connection_closed(id, ci, cp);
    }

    fn inject_address_change(
        &mut self,
        id: &PeerId,
        ci: &ConnectionId,
        old: &ConnectedPoint,
        new: &ConnectedPoint,
    ) {
        self.dht.inject_address_change(id, ci, old, new);
    }
}
