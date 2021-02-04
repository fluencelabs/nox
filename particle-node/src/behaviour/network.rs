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
use server_config::NetworkConfig;

use libp2p::{
    identify::Identify,
    identity::PublicKey,
    ping::{Ping, PingConfig, PingEvent},
    PeerId, Swarm,
};

use crate::network_api::NetworkApi;
use aquamarine::{SendParticle, StepperEffects};
use async_std::sync::Mutex;
use async_std::task::JoinHandle;
use connection_pool::{ConnectionPoolBehaviour, ConnectionPoolInlet, ConnectionPoolT, Contact};
use fluence_libp2p::types::BackPressuredInlet;
use futures::select;
use futures::StreamExt;
use kademlia::{Kademlia, KademliaApi, KademliaApiInlet, KademliaConfig};
use libp2p::swarm::ExpandedSwarm;
use particle_protocol::Particle;
use std::sync::Arc;
use std::task::Poll;

pub type SwarmEventType = generate_swarm_event_type!(NetworkBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
pub struct NetworkBehaviour {
    // TODO: move identify inside ConnectionPoolBehaviour?
    identity: Identify,
    // TODO: move ping inside ConnectionPoolBehaviour?
    ping: Ping,
    pub(crate) connection_pool: ConnectionPoolInlet,
    pub(crate) kademlia: KademliaApiInlet,
}

impl NetworkBehaviour {
    pub fn new(cfg: NetworkConfig) -> anyhow::Result<(Self, NetworkApi)> {
        let local_public_key = PublicKey::Ed25519(cfg.key_pair.public());
        let identity = Identify::new(
            "/fluence/faas/1.0.0".into(),
            "0.1.0".into(),
            local_public_key,
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        let kad_config = KademliaConfig {
            peer_id: cfg.local_peer_id,
            keypair: cfg.key_pair,
            kad_config: cfg.kademlia_config,
        };

        // TODO: this is hazy; names are bad, conversion is far from transparent. Hide behaviours?
        let kademlia = Kademlia::new(kad_config, cfg.trust_graph, cfg.registry.as_ref());
        let (kademlia_api, kademlia) = kademlia.into();
        let (connection_pool, particle_stream) = ConnectionPoolBehaviour::new(
            cfg.particle_queue_buffer,
            cfg.protocol_config,
            cfg.local_peer_id,
        );
        let (connection_pool_api, connection_pool) = connection_pool.into();

        Ok((
            Self {
                kademlia,
                connection_pool,
                identity,
                ping,
            },
            NetworkApi::new(
                particle_stream,
                cfg.particle_parallelism,
                kademlia_api,
                connection_pool_api,
            ),
        ))
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for NetworkBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for NetworkBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
