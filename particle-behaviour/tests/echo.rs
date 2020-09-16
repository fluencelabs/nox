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

use fluence_libp2p::{build_memory_transport, RandomPeerId};
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput;
use libp2p::core::transport::dummy::{DummyStream, DummyTransport};
use libp2p::core::Multiaddr;
use libp2p::mplex::Multiplex;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{PeerId, Swarm};
use particle_behaviour::ParticleBehaviour;
use particle_dht::DHTConfig;
use particle_protocol::{Particle, ProtocolMessage};
use trust_graph::ed25519::Keypair;
use trust_graph::TrustGraph;

#[test]
fn echo_particle() {
    let (mut swarm1, addr1, peer_id1) = make_swarm();
    let (mut swarm2, addr2, peer_id2) = make_swarm();

    Swarm::dial_addr(&mut swarm1, addr2).expect("connect");
    swarm1.inject_event(
        peer_id2.clone(),
        ConnectionId::new(0),
        EitherOutput::First(ProtocolMessage::Particle(Particle {
            id: "123".to_string(),
            init_peer_id: peer_id2.clone(),
            timestamp: 0,
            ttl: 1,
            script: "".to_string(),
            signature: vec![],
            data: Default::default(),
        })),
    );
}

fn make_swarm() -> (Swarm<ParticleBehaviour>, Multiaddr, PeerId) {
    use libp2p::identity::Keypair::Ed25519;

    let keypair = Keypair::generate();
    let public = libp2p::identity::PublicKey::Ed25519(keypair.public());
    let peer_id = public.clone().into_peer_id();
    let config = DHTConfig {
        peer_id: peer_id.clone(),
        keypair: keypair.clone(),
    };
    let trust_graph = TrustGraph::new(<_>::default());
    let registry = None;
    let behaviour = ParticleBehaviour::new(config, trust_graph, registry);

    let transport = build_memory_transport(Ed25519(keypair));
    let mut swarm = Swarm::new(transport, behaviour, peer_id.clone());

    let address = create_memory_maddr();
    Swarm::listen_on(&mut swarm, address.clone()).expect("listen");

    (swarm, address, peer_id)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}
