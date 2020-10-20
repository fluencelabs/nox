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

#![recursion_limit = "256"]

use particle_behaviour::{ParticleBehaviour, ParticleConfig};
use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};

use fluence_libp2p::{build_memory_transport, generate_swarm_event_type};
use test_utils::{make_tmp_dir, now, put_aquamarine, timeout, TIMEOUT};
use trust_graph::TrustGraph;

use async_std::task;
use futures::{
    future::FutureExt,
    select,
    task::{Context, Poll},
};
use libp2p::{
    core::{connection::ConnectionId, Multiaddr},
    identity::ed25519::Keypair,
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
        SwarmEvent,
    },
    PeerId, Swarm,
};
use serde_json::json;
use std::{collections::VecDeque, time::Duration};

#[test]
fn echo_particle() {
    let (mut server, addr, server_id) = make_server();
    let (mut client, client_id) = make_client(addr);

    // Poll server & client in background
    // Once client connected, send Particle
    // Exit when server echoes the particle
    let particle = task::block_on(timeout(
        TIMEOUT,
        task::spawn(async move {
            loop {
                select!(
                    event = server.next_event().fuse() => {},
                    event = client.next_event().fuse() => {
                        println!("client got event: {:?}", event);
                        match event {
                            SwarmEvent::ConnectionEstablished { .. } => {
                                let p = Particle {
                                    id: "123".to_string(),
                                    init_peer_id: client_id.clone(),
                                    timestamp: now(),
                                    ttl: 100,
                                    script: format!(r#"(call ("{}" ("a" "b") (data) void))"#, client_id),
                                    signature: vec![],
                                    data: json!({"data": "none"}),
                                };
                                client.send(p.clone(), server_id.clone());
                            }
                            SwarmEvent::Behaviour(particle) => {
                                break particle
                            }
                            _ => {}
                        }
                    }
                )
            }
        }),
    )).expect("timed out");

    assert_eq!(particle.id, "123".to_string());
    assert_eq!(particle.data["data"], json!("none"));
}

macro_rules! make_swarm {
    ($behaviour:expr) => {{
        use libp2p::identity::Keypair::Ed25519;

        let keypair = Keypair::generate();
        let public = libp2p::identity::PublicKey::Ed25519(keypair.public());
        let peer_id = public.clone().into_peer_id();

        let behaviour = $behaviour(peer_id.clone(), keypair.clone());
        let transport = build_memory_transport(Ed25519(keypair));
        let swarm = Swarm::new(transport, behaviour, peer_id.clone());

        swarm
    }};
}

fn make_server() -> (Swarm<ParticleBehaviour>, Multiaddr, PeerId) {
    let mut swarm = make_swarm!(|peer_id: PeerId, keypair: Keypair| {
        let tmp = make_tmp_dir();
        put_aquamarine(tmp.join("modules"), None);
        let trust_graph = TrustGraph::new(<_>::default());
        let registry = None;
        let tout = Duration::from_secs(100);
        let config = ProtocolConfig::new(tout.clone(), tout.clone(), tout);
        let config = ParticleConfig::new(
            config,
            peer_id,
            tmp.clone(),
            <_>::default(),
            tmp.clone(),
            keypair,
            1,
        );
        let behaviour =
            ParticleBehaviour::new(config, trust_graph, registry).expect("particle behaviour");
        behaviour
    });

    let address = create_memory_maddr();
    Swarm::listen_on(&mut swarm, address.clone()).expect("listen");

    let peer_id = Swarm::local_peer_id(&swarm).clone();
    (swarm, address, peer_id)
}

fn make_client(address: Multiaddr) -> (Swarm<Client>, PeerId) {
    let mut swarm = make_swarm!(|_, _| Client::new());
    Swarm::dial_addr(&mut swarm, address).expect("dial");

    let peer_id = Swarm::local_peer_id(&swarm).clone();
    (swarm, peer_id)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

type ClientEventType = generate_swarm_event_type!(Client);

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
        ProtocolConfig::new(
            Duration::from_secs(100),
            Duration::from_secs(100),
            Duration::from_secs(100),
        )
        .into()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_event(&mut self, peer_id: PeerId, _: ConnectionId, event: ProtocolMessage) {
        match event {
            ProtocolMessage::Particle(p) => self
                .events
                .push_back(NetworkBehaviourAction::GenerateEvent(p)),
            ProtocolMessage::UpgradeError(_) => {}
            ProtocolMessage::Upgrade => println!("client got connection! {}", peer_id),
        }
    }

    fn poll(&mut self, _: &mut Context<'_>, _: &mut impl PollParameters) -> Poll<ClientEventType> {
        if let Some(e) = self.events.pop_front() {
            return Poll::Ready(e);
        }

        Poll::Pending
    }
}
