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

use fluence_libp2p::generate_swarm_event_type;
use particle_actors::Plumber;
use particle_dht::{DHTConfig, ParticleDHT};
use trust_graph::TrustGraph;

use libp2p::{
    core::{
        connection::{ConnectedPoint, ConnectionId, ListenerId},
        either::EitherOutput,
        Multiaddr,
    },
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent},
    swarm::{
        IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use prometheus::Registry;
use std::error::Error;
use std::task::{Context, Poll, Waker};

macro_rules! poll_loop {
    ($self:ident,$behaviour:expr,$cx:expr,$params:expr,$either:path) => {{
        loop {
            match NetworkBehaviour::poll(&mut $behaviour, $cx, $params) {
                Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)) => {
                    NBEP::inject_event($self, event)
                }
                Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    event,
                    handler,
                }) => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        event: $either(event),
                        handler,
                    })
                }
                Poll::Ready(NetworkBehaviourAction::DialAddress { address }) => {
                    return Poll::Ready(NetworkBehaviourAction::DialAddress { address })
                }
                Poll::Ready(NetworkBehaviourAction::ReportObservedAddr { address }) => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr { address })
                }
                Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition }) => {
                    return Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition })
                }
                Poll::Pending => break,
            }
        }
    }};
}

pub(crate) type SwarmEventType = generate_swarm_event_type!(ParticleBehaviour);

pub struct ParticleBehaviour {
    plumber: Plumber,
    dht: ParticleDHT,
    waker: Option<Waker>,
}

impl libp2p::swarm::NetworkBehaviourEventProcess<KademliaEvent> for ParticleBehaviour {
    fn inject_event(&mut self, event: KademliaEvent) {
        unimplemented!()
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ParticleBehaviour {
    fn inject_event(&mut self, event: ()) {
        unimplemented!()
    }
}

impl ParticleBehaviour {
    pub fn new(config: DHTConfig, trust_graph: TrustGraph, registry: Option<&Registry>) -> Self {
        let plumber = Plumber::new();
        let dht = ParticleDHT::new(config, trust_graph, registry);

        Self {
            plumber,
            dht,
            waker: None,
        }
    }
}

impl NetworkBehaviour for ParticleBehaviour {
    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        <Plumber as NetworkBehaviour>::ProtocolsHandler,
        <ParticleDHT as NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(self.plumber.new_handler(), self.dht.new_handler())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        let p = self.plumber.addresses_of_peer(peer_id).into_iter();
        let d = self.dht.addresses_of_peer(peer_id).into_iter();

        p.chain(d).collect()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        self.plumber.inject_connected(peer_id);
        self.dht.inject_connected(peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.plumber.inject_disconnected(peer_id);
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
            First(event) => self.plumber.inject_event(peer_id, connection, event),
            Second(event) => {
                NetworkBehaviour::inject_event(&mut self.dht, peer_id, connection, event)
            }
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        use NetworkBehaviourAction::*;
        use NetworkBehaviourEventProcess as NBEP;

        self.waker = Some(cx.waker().clone());

        poll_loop!(self, self.plumber, cx, params, EitherOutput::First);
        poll_loop!(self, self.dht, cx, params, EitherOutput::Second);

        Poll::Pending
    }

    // ==== useless repetition below ====
    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        addr: &Multiaddr,
        error: &dyn Error,
    ) {
        self.plumber.inject_addr_reach_failure(peer_id, addr, error);
        self.dht.inject_addr_reach_failure(peer_id, addr, error);
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        self.plumber.inject_dial_failure(peer_id);
        self.dht.inject_dial_failure(peer_id);
    }

    fn inject_new_listen_addr(&mut self, addr: &Multiaddr) {
        self.plumber.inject_new_listen_addr(addr);
        self.dht.inject_new_listen_addr(addr);
    }

    fn inject_expired_listen_addr(&mut self, addr: &Multiaddr) {
        self.plumber.inject_expired_listen_addr(addr);
        self.dht.inject_expired_listen_addr(addr);
    }

    fn inject_new_external_addr(&mut self, addr: &Multiaddr) {
        self.plumber.inject_new_external_addr(addr);
        self.dht.inject_new_external_addr(addr);
    }

    fn inject_listener_error(&mut self, id: ListenerId, err: &(dyn std::error::Error + 'static)) {
        self.plumber.inject_listener_error(id, err);
        self.dht.inject_listener_error(id, err);
    }

    fn inject_listener_closed(&mut self, id: ListenerId, reason: Result<(), &std::io::Error>) {
        self.plumber.inject_listener_closed(id, reason);
        self.dht.inject_listener_closed(id, reason);
    }

    fn inject_connection_established(
        &mut self,
        id: &PeerId,
        ci: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        self.plumber.inject_connection_established(id, ci, cp);
        self.dht.inject_connection_established(id, ci, cp);
    }

    fn inject_connection_closed(&mut self, id: &PeerId, ci: &ConnectionId, cp: &ConnectedPoint) {
        self.plumber.inject_connection_closed(id, ci, cp);
        self.dht.inject_connection_closed(id, ci, cp);
    }

    fn inject_address_change(
        &mut self,
        id: &PeerId,
        ci: &ConnectionId,
        old: &ConnectedPoint,
        new: &ConnectedPoint,
    ) {
        self.plumber.inject_address_change(id, ci, old, new);
        self.dht.inject_address_change(id, ci, old, new);
    }
}
