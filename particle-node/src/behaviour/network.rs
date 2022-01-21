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
use libp2p::identify::IdentifyConfig;
use libp2p::{
    identify::Identify,
    ping::{Ping, PingConfig, PingEvent},
};

use connection_pool::{ConnectionPoolBehaviour, ConnectionPoolInlet};
use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, Inlet};
use kademlia::{Kademlia, KademliaApiInlet, KademliaConfig};
use particle_protocol::{Particle, PROTOCOL_NAME};
use server_config::NetworkConfig;

use crate::connectivity::Connectivity;

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct NetworkBehaviour {
    identify: Identify,
    ping: Ping,
    pub(crate) connection_pool: ConnectionPoolInlet,
    pub(crate) kademlia: KademliaApiInlet,
    #[behaviour(ignore)]
    /// Whether to allow local (127.0.0.1) addresses in identify
    pub(super) allow_local_addresses: bool,
}

impl NetworkBehaviour {
    pub fn new(cfg: NetworkConfig) -> (Self, Connectivity, BackPressuredInlet<Particle>) {
        let local_public_key = cfg.key_pair.public();
        let identify = Identify::new(
            IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key)
                .with_agent_version(cfg.node_version.into()),
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        let kad_config = KademliaConfig {
            peer_id: cfg.local_peer_id,
            keypair: cfg.key_pair,
            kad_config: cfg.kademlia_config,
        };

        let kademlia = Kademlia::new(kad_config, cfg.libp2p_metrics);
        let (kademlia_api, kademlia) = kademlia.into();
        let (connection_pool, particle_stream) = ConnectionPoolBehaviour::new(
            cfg.particle_queue_buffer,
            cfg.protocol_config,
            cfg.local_peer_id,
        );
        let (connection_pool_api, connection_pool) = connection_pool.into();

        let this = Self {
            kademlia,
            connection_pool,
            identify,
            ping,
            allow_local_addresses: cfg.allow_local_addresses,
        };

        let connectivity = Connectivity {
            peer_id: cfg.local_peer_id,
            kademlia: kademlia_api,
            connection_pool: connection_pool_api,
            bootstrap_nodes: cfg.bootstrap_nodes.into_iter().collect(),
            bootstrap_frequency: cfg.bootstrap_frequency,
        };

        (this, connectivity, particle_stream)
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for NetworkBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for NetworkBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
