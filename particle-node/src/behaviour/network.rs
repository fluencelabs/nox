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

use crate::bootstrapper::Bootstrapper;

use fluence_libp2p::generate_swarm_event_type;
use server_config::BehaviourConfig;

use libp2p::{
    identify::Identify,
    identity::PublicKey,
    ping::{Ping, PingConfig, PingEvent},
    PeerId, Swarm,
};

use crate::network_api::NetworkApi;
use crate::node::unlocks::{unlock, unlock_f};
use async_std::sync::Mutex;
use async_std::task::JoinHandle;
use connection_pool::{ConnectionPool, ConnectionPoolBehaviour, Contact};
use fluence_libp2p::types::BackPressuredInlet;
use futures::select;
use futures::StreamExt;
use kademlia::{Kademlia, KademliaApiInlet, KademliaApiOutlet, KademliaConfig};
use libp2p::swarm::ExpandedSwarm;
use particle_actors::{SendParticle, StepperEffects};
use particle_protocol::Particle;
use std::sync::Arc;
use std::task::Poll;

pub type SwarmEventType = generate_swarm_event_type!(NetworkBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
pub struct NetworkBehaviour {
    // TODO: move bootstrapper inside kademlia?
    bootstrapper: Bootstrapper,
    // TODO: move identify inside ConnectionPoolBehaviour?
    identity: Identify,
    // TODO: move ping inside ConnectionPoolBehaviour?
    ping: Ping,
    pub(crate) connection_pool: ConnectionPoolBehaviour,
    pub(crate) kademlia: KademliaApiInlet,
}

impl NetworkBehaviour {
    pub fn new(cfg: BehaviourConfig<'_>) -> anyhow::Result<(Self, NetworkApi)> {
        let local_public_key = PublicKey::Ed25519(cfg.key_pair.public());
        let identity = Identify::new(
            "/fluence/faas/1.0.0".into(),
            "0.1.0".into(),
            local_public_key,
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        let kad_config = KademliaConfig {
            peer_id: cfg.local_peer_id.clone(),
            keypair: cfg.key_pair,
            kad_config: cfg.kademlia_config,
        };
        let kademlia = Kademlia::new(kad_config, cfg.trust_graph, cfg.registry);
        let (kademlia_api, kademlia) = kademlia.into();
        let (connection_pool, particle_stream) =
            ConnectionPoolBehaviour::new(cfg.particle_queue_buffer, cfg.protocol_config);

        let bootstrapper = Bootstrapper::new(cfg.bootstrap, cfg.local_peer_id, cfg.bootstrap_nodes);

        Ok((
            Self {
                kademlia,
                connection_pool,
                identity,
                ping,
                bootstrapper,
            },
            NetworkApi {
                particle_stream,
                kademlia_api,
            },
        ))
    }

    /// Dials bootstrap nodes
    pub fn dial_bootstrap_nodes(&mut self) {
        // // TODO: how to avoid collect?
        // let bootstrap_nodes: Vec<_> = self.bootstrapper.bootstrap_nodes.iter().cloned().collect();
        // if bootstrap_nodes.is_empty() {
        //     log::warn!("No bootstrap nodes found. Am I the only one? :(");
        // }
        // for maddr in bootstrap_nodes {
        //     self.dial(maddr)
        // }

        todo!("dial bootstrap nodes")
    }

    pub fn bootstrap(&mut self) {
        // self.particle.bootstrap()
        todo!("bootstrap? or delete")
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for NetworkBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for NetworkBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
