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

use particle_actors::{Plumber, PlumberEvent};
use particle_dht::{DHTEvent, ParticleDHT};
use particle_protocol::{Particle, ProtocolMessage};
use particle_services::ParticleServices;

use fluence_libp2p::generate_swarm_event_type;
use trust_graph::TrustGraph;

use crate::clients::PeerKind;
use crate::ParticleConfig;
use libp2p::{
    core::{either::EitherOutput, identity::ed25519, Multiaddr},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use prometheus::Registry;
use std::{
    collections::{HashMap, VecDeque},
    io,
    task::Waker,
};

pub(crate) type SwarmEventType = generate_swarm_event_type!(ParticleBehaviour);

pub struct ParticleBehaviour {
    pub(super) plumber: Plumber,
    pub(super) dht: ParticleDHT,
    #[allow(dead_code)]
    pub(super) services: ParticleServices,
    pub(super) clients: HashMap<PeerId, Option<Multiaddr>>,
    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ParticleBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<DHTEvent> for ParticleBehaviour {
    fn inject_event(&mut self, event: DHTEvent) {
        log::info!("DHT event: {:?}", event);
        match event {
            DHTEvent::Published(_) => {}
            DHTEvent::PublishFailed(_, _) => {}
            DHTEvent::Forward { target, particle } => self.forward_particle(target, particle),
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PlumberEvent> for ParticleBehaviour {
    fn inject_event(&mut self, event: PlumberEvent) {
        log::info!("Plumber event: {:?}", event);
        match event {
            PlumberEvent::Forward { target, particle } => {
                if let PeerKind::Client = self.peer_kind(&target) {
                    self.forward_particle(target, particle);
                } else {
                    self.dht.send_to(target, particle)
                }
            }
        }
    }
}

impl ParticleBehaviour {
    pub fn new(
        config: ParticleConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> io::Result<Self> {
        let services = ParticleServices::new(config.services_config()?);
        let plumber = Plumber::new(config.actor_config()?, services.fabric());
        let dht = ParticleDHT::new(config.dht_config(), trust_graph, registry);

        Ok(Self {
            plumber,
            dht,
            services,
            clients: <_>::default(),
            events: <_>::default(),
            waker: <_>::default(),
        })
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

    pub(super) fn push_event(&mut self, event: SwarmEventType) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }

        self.events.push_back(event);
    }

    fn forward_particle(&mut self, target: PeerId, particle: Particle) {
        self.push_event(NetworkBehaviourAction::NotifyHandler {
            peer_id: target,
            handler: NotifyHandler::Any,
            event: EitherOutput::First(ProtocolMessage::Particle(particle)),
        });
    }
}
