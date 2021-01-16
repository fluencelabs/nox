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

use crate::identify::identify;
use crate::ParticleConfig;

use particle_actors::Plumber;
use particle_dht::ParticleDHT;
use particle_protocol::{HandlerMessage, Particle, ProtocolConfig};
use particle_services::ParticleAppServices;

use fluence_libp2p::generate_swarm_event_type;
use trust_graph::TrustGraph;

use libp2p::{
    core::{either::EitherOutput, identity::ed25519, Multiaddr},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use particle_closures::{HostClosures, Mailbox};
use particle_providers::ProviderRepository;
use prometheus::Registry;
use std::{
    collections::{HashMap, VecDeque},
    task::Waker,
};

pub(crate) type SwarmEventType = generate_swarm_event_type!(ParticleBehaviour);

pub struct ParticleBehaviour {
    pub(super) plumber: Plumber,
    pub(super) dht: ParticleDHT,
    #[allow(dead_code)]
    services: ParticleAppServices,
    pub(super) mailbox: Mailbox,
    #[allow(dead_code)]
    providers: ProviderRepository,
    pub(super) connected_peers: HashMap<PeerId, Multiaddr>,
    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ParticleBehaviour {
    pub fn new(
        config: ParticleConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> anyhow::Result<Self> {
        let services = ParticleAppServices::new(config.services_config()?);
        let mailbox = Mailbox::new();
        let providers = ProviderRepository::new(config.current_peer_id.clone());
        let closures = HostClosures {
            add_provider: providers.add_provider(),
            get_providers: providers.get_providers(),
            get_modules: particle_modules::get_modules(config.modules_dir()?),
            get_blueprints: particle_modules::get_blueprints(config.blueprint_dir()?),
            add_module: particle_modules::add_module(config.modules_dir()?),
            add_blueprint: particle_modules::add_blueprint(config.blueprint_dir()?),
            create_service: services.create_service(),
            call_service: services.call_service(),
            get_interface: services.get_interface(),
            get_active_interfaces: services.get_active_interfaces(),
            resolve: mailbox.get_api().resolve(),
            neighborhood: mailbox.get_api().neighborhood(),
            identify: identify(config.node_info.clone()),
        };
        let plumber = Plumber::new(config.actor_config()?, closures.descriptor());
        let dht = ParticleDHT::new(config.dht_config(), trust_graph, registry);

        Ok(Self {
            plumber,
            dht,
            services,
            mailbox,
            providers,
            protocol_config: config.protocol_config,
            connected_peers: <_>::default(),
            events: <_>::default(),
            waker: <_>::default(),
        })
    }

    /// Called on Identify event. That's when we got remote node's public key
    /// TODO: why not take public key from PeerId and get rid of identify altogether?
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
        self.wake();
        self.events.push_back(event);
    }

    pub(super) fn forward_particle(&mut self, target: PeerId, particle: Particle) {
        log::info!("Sending {} to {} directly", particle.id, target);
        self.push_event(NetworkBehaviourAction::NotifyHandler {
            peer_id: target,
            handler: NotifyHandler::Any,
            event: EitherOutput::First(HandlerMessage::OutParticle(particle, <_>::default())),
        });
    }

    pub(super) fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}
