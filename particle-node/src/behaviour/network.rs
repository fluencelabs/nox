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
use libp2p::identify::{IdentifyConfig, IdentifyEvent};
use libp2p::{
    identify::Identify,
    ping::{Ping, PingConfig, PingEvent},
    swarm::NetworkBehaviour,
};

use connection_pool::ConnectionPoolBehaviour;
use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{Kademlia, KademliaConfig};
use particle_protocol::{Particle, PROTOCOL_NAME};
use server_config::NetworkConfig;

use crate::connectivity::Connectivity;

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
pub struct FluenceNetworkBehaviour {
    identify: Identify,
    ping: Ping,
    pub(crate) connection_pool: ConnectionPoolBehaviour,
    pub(crate) kademlia: Kademlia,
    // #[behaviour(ignore)]
    // /// Whether to allow private IP addresses in identify
    // pub(super) allow_local_addresses: bool,
}

impl FluenceNetworkBehaviour {
    pub fn new(cfg: NetworkConfig) -> (Self, Connectivity, BackPressuredInlet<Particle>) {
        let local_public_key = cfg.key_pair.public();
        let identify = Identify::new(
            IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key)
                .with_agent_version(cfg.node_version.into()),
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        let kad_config = KademliaConfig {
            peer_id: cfg.local_peer_id,
            kad_config: cfg.kademlia_config,
        };

        let (kademlia, kademlia_api) = Kademlia::new(kad_config, cfg.libp2p_metrics);
        let (connection_pool, particle_stream, connection_pool_api) = ConnectionPoolBehaviour::new(
            cfg.particle_queue_buffer,
            cfg.protocol_config,
            cfg.local_peer_id,
            cfg.connection_pool_metrics,
        );

        let this = Self {
            kademlia,
            connection_pool,
            identify,
            ping,
        };

        let connectivity = Connectivity {
            peer_id: cfg.local_peer_id,
            kademlia: kademlia_api,
            connection_pool: connection_pool_api,
            bootstrap_nodes: cfg.bootstrap_nodes.into_iter().collect(),
            bootstrap_frequency: cfg.bootstrap_frequency,
            metrics: cfg.connectivity_metrics,
        };

        (this, connectivity, particle_stream)
    }
}
