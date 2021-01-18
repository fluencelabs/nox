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

use connection_pool::ConnectionPoolBehaviour;
use kademlia::Kademlia;
use particle_actors::Plumber;
use particle_protocol::{HandlerMessage, Particle, ProtocolConfig};
use particle_services::ParticleAppServices;

use fluence_libp2p::generate_swarm_event_type;
use trust_graph::TrustGraph;

use fluence_libp2p::types::BackPressuredInlet;
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
    pub(super) connection_pool: ConnectionPoolBehaviour,
    pub(super) kademlia: Kademlia,

    #[allow(dead_code)]
    pub(super) plumber: Plumber,
    #[allow(dead_code)]
    services: ParticleAppServices,
    #[allow(dead_code)]
    providers: ProviderRepository,
    pub(super) waker: Option<Waker>,
}

impl ParticleBehaviour {
    pub fn new(
        config: ParticleConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> anyhow::Result<(Self, BackPressuredInlet<Particle>)> {
        let services = ParticleAppServices::new(config.services_config()?);
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
            identify: identify(config.node_info.clone()),
        };
        let plumber = Plumber::new(config.actor_config()?, closures.descriptor());

        let kademlia = Kademlia::new(config.dht_config(), trust_graph, registry);
        let (connection_pool, particle_stream) =
            ConnectionPoolBehaviour::new(config.particle_queue_buffer, config.protocol_config);

        Ok((
            Self {
                connection_pool,
                kademlia,
                plumber,
                services,
                providers,
                waker: <_>::default(),
            },
            particle_stream,
        ))
    }

    /// Called on Identify event. That's when we got remote node's public key
    /// TODO: why not take public key from PeerId and get rid of identify altogether?
    pub fn add_kad_node(
        &mut self,
        _node_id: PeerId,
        _addresses: Vec<Multiaddr>,
        _public_key: ed25519::PublicKey,
    ) {
        // self.dht.add_kad_node(node_id, addresses, public_key)
    }

    pub(super) fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}
