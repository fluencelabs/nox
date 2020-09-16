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

use async_std::task;
use fluence_libp2p::{build_memory_transport, generate_swarm_event_type, RandomPeerId};
use futures::channel::mpsc;
use futures::future::FutureExt;
use futures::select;
use futures::task::{Context, Poll};
use futures::StreamExt;
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput;
use libp2p::core::transport::dummy::{DummyStream, DummyTransport};
use libp2p::core::Multiaddr;
use libp2p::identity::ed25519::Keypair;
use libp2p::mplex::Multiplex;
use libp2p::swarm::{
    NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
    SwarmEvent,
};
use libp2p::{PeerId, Swarm};
use particle_behaviour::ParticleBehaviour;
use particle_dht::DHTConfig;
use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};
use std::collections::VecDeque;
use trust_graph::TrustGraph;

#[test]
fn echo_particle() {
    let (mut swarm, addr, swarm_id) = make_server();
    let (mut client, client_id) = make_client(addr);

    let particle = task::block_on(task::spawn(async move {
        loop {
            select!(
                event = swarm.next_event().fuse() => println!("swarm event: {:?}", event),
                event = client.next_event().fuse() => {
                    println!("client got event: {:?}", event);
                    match event {
                        SwarmEvent::ConnectionEstablished { .. } => {
                            let p = Particle {
                                id: "123".to_string(),
                                init_peer_id: client_id.clone(),
                                timestamp: 0,
                                ttl: 1,
                                script: "".to_string(),
                                signature: vec![],
                                data: Default::default(),
                            };
                            client.send(p.clone(), swarm_id.clone());
                        }
                        SwarmEvent::Behaviour(particle) => {
                            break particle
                        }
                        _ => {}
                    }
                }
            )
        }
    }));

    assert_eq!(particle.id, "123".to_string())
}

macro_rules! make_swarm {
    ($behaviour:expr) => {{
        use libp2p::identity::Keypair::Ed25519;

        let keypair = Keypair::generate();
        let public = libp2p::identity::PublicKey::Ed25519(keypair.public());
        let peer_id = public.clone().into_peer_id();

        let behaviour = $behaviour(peer_id.clone(), keypair.clone());
        let transport = build_memory_transport(Ed25519(keypair));
        let mut swarm = Swarm::new(transport, behaviour, peer_id.clone());

        swarm
    }};
}

fn make_server() -> (Swarm<ParticleBehaviour>, Multiaddr, PeerId) {
    let mut swarm = make_swarm!(|peer_id: PeerId, keypair: Keypair| {
        let config = DHTConfig {
            peer_id: peer_id.clone(),
            keypair: keypair.clone(),
        };
        let trust_graph = TrustGraph::new(<_>::default());
        let registry = None;
        ParticleBehaviour::new(config, trust_graph, registry)
    });

    let address = create_memory_maddr();
    Swarm::listen_on(&mut swarm, address.clone()).expect("listen");

    let peer_id = Swarm::local_peer_id(&mut swarm).clone();
    (swarm, address, peer_id)
}

fn make_client(address: Multiaddr) -> (Swarm<Client>, PeerId) {
    let mut swarm = make_swarm!(|_, _| Client::new());
    Swarm::dial_addr(&mut swarm, address).expect("dial");

    let peer_id = Swarm::local_peer_id(&mut swarm).clone();
    (swarm, peer_id)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

pub type ClientEventType = generate_swarm_event_type!(Client);

#[derive(Default)]
struct Client {
    events: VecDeque<ClientEventType>,
}

impl Client {
    pub fn new() -> Self {
        <_>::default()
    }

    pub fn send(&mut self, p: Particle, target: PeerId) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: target,
                handler: NotifyHandler::Any,
                event: ProtocolMessage::Particle(p),
            })
    }
}

impl NetworkBehaviour for Client {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>;
    type OutEvent = Particle;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        ProtocolConfig::new().into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, peer_id: &PeerId) {}

    fn inject_event(&mut self, peer_id: PeerId, connection: ConnectionId, event: ProtocolMessage) {
        match event {
            ProtocolMessage::Particle(p) => self
                .events
                .push_back(NetworkBehaviourAction::GenerateEvent(p)),
            ProtocolMessage::UpgradeError(_) => {}
            ProtocolMessage::Upgrade => println!("client got connection! {}", peer_id),
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<ClientEventType> {
        if let Some(e) = self.events.pop_front() {
            return Poll::Ready(e);
        }

        Poll::Pending
    }
}
