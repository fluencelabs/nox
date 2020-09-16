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

use fluence_libp2p::{generate_swarm_event_type, poll_loop};
use particle_actors::{Plumber, PlumberEvent};
use particle_dht::{DHTConfig, DHTEvent, ParticleDHT};
use trust_graph::TrustGraph;

use libp2p::core::identity::ed25519;
use libp2p::swarm::NotifyHandler;
use libp2p::{
    core::{
        connection::{ConnectedPoint, ConnectionId, ListenerId},
        either::EitherOutput,
        Multiaddr,
    },
    swarm::{
        IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
        OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use particle_protocol::{ProtocolConfig, ProtocolMessage};
use prometheus::Registry;
use std::error::Error;
use std::task::{Context, Poll, Waker};

pub(crate) type SwarmEventType = generate_swarm_event_type!(ParticleBehaviour);

pub struct ParticleBehaviour {
    plumber: Plumber,
    dht: ParticleDHT,
    waker: Option<Waker>,
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ParticleBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<DHTEvent> for ParticleBehaviour {
    fn inject_event(&mut self, _event: DHTEvent) {}
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

    pub fn add_kad_node(
        &mut self,
        node_id: PeerId,
        addresses: Vec<Multiaddr>,
        public_key: ed25519::PublicKey,
    ) {
        self.dht.add_kad_node(node_id, addresses, public_key)
    }

    pub fn bootstrap(&mut self) {
        self.dht.bootstrap()
    }
}

impl NetworkBehaviour for ParticleBehaviour {
    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>,
        <ParticleDHT as NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(ProtocolConfig::new().into(), self.dht.new_handler())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        let p = self.plumber.client_address(peer_id).clone().into_iter();
        let d = self.dht.addresses_of_peer(peer_id).into_iter();

        p.chain(d).collect()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        self.dht.inject_connected(peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
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
                ProtocolMessage::Upgrade => {
                    self.plumber.add_client(peer_id.clone());
                    self.dht.publish_client(peer_id);
                }
                ProtocolMessage::Particle(particle) => self.plumber.ingest(particle),
                ProtocolMessage::UpgradeError(_) => {}
            },
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
        self.waker = Some(cx.waker().clone());

        if let Poll::Ready(event) = self.plumber.poll() {
            match event {
                PlumberEvent::Forward { target, particle } => {
                    // TODO: check if target is a client in Plumber
                    //       resolve target through DHT otherwise
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id: target,
                        handler: NotifyHandler::Any,
                        event: EitherOutput::First(ProtocolMessage::Particle(particle)),
                    });
                }
            }
        }

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
