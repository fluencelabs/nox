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

use crate::ParticleConfig;

use particle_actors::Plumber;
use particle_dht::ParticleDHT;
use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};
use particle_services::ParticleAppServices;

use fluence_libp2p::generate_swarm_event_type;
use trust_graph::TrustGraph;

use libp2p::{
    core::{either::EitherOutput, identity::ed25519, Multiaddr},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use particle_closures::{HostClosures, Mailbox};
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
    pub(super) services: ParticleAppServices,
    #[allow(dead_code)]
    pub(super) mailbox: Mailbox,
    pub(super) clients: HashMap<PeerId, Multiaddr>,
    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ParticleBehaviour {
    pub fn new(
        config: ParticleConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> io::Result<Self> {
        let services = ParticleAppServices::new(config.services_config()?);
        let mailbox = Mailbox::new();
        let closures = HostClosures {
            add_module: particle_modules::add_module(config.modules_dir()),
            add_blueprint: particle_modules::add_blueprint(config.blueprint_dir()),
            create_service: services.create_service(),
            call_service: services.call_service(),
            builtin: mailbox.get_api().router(),
        };
        let plumber = Plumber::new(config.actor_config()?, closures.descriptor());
        let dht = ParticleDHT::new(config.dht_config(), trust_graph, registry);

        Ok(Self {
            plumber,
            dht,
            services,
            mailbox,
            protocol_config: config.protocol_config,
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

    pub(super) fn forward_particle(&mut self, target: PeerId, particle: Particle) {
        self.push_event(NetworkBehaviourAction::NotifyHandler {
            peer_id: target,
            handler: NotifyHandler::Any,
            event: EitherOutput::First(ProtocolMessage::Particle(particle)),
        });
    }
}
