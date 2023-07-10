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
use libp2p::identify::Config as IdentifyConfig;
use libp2p::{
    connection_limits::Behaviour as ConnectionLimits,
    identify::Behaviour as Identify,
    ping::{Behaviour as Ping, Config as PingConfig},
    swarm::NetworkBehaviour,
};
use tokio::sync::mpsc;

use connection_pool::ConnectionPoolBehaviour;
use health::HealthCheckRegistry;
use kademlia::{Kademlia, KademliaConfig};
use particle_protocol::{Particle, PROTOCOL_NAME};
use server_config::NetworkConfig;

use crate::connectivity::{BootstrapNodesHealthCheck, Connectivity, ConnectivityHealthChecks};

/// Coordinates protocols, so they can cooperate
#[derive(NetworkBehaviour)]
pub struct FluenceNetworkBehaviour {
    identify: Identify,
    ping: Ping,
    connection_limits: ConnectionLimits,
    pub(crate) connection_pool: ConnectionPoolBehaviour,
    pub(crate) kademlia: Kademlia,
}

impl FluenceNetworkBehaviour {
    pub fn new(
        cfg: NetworkConfig,
        health_registry: Option<&mut HealthCheckRegistry>,
    ) -> (Self, Connectivity, mpsc::Receiver<Particle>) {
        let local_public_key = cfg.key_pair.public();
        let identify = Identify::new(
            IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key)
                .with_agent_version(cfg.node_version.into()),
        );
        let ping = Ping::new(PingConfig::new());

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

        let connection_limits = ConnectionLimits::new(cfg.connection_limits);

        let this = Self {
            kademlia,
            connection_pool,
            connection_limits,
            identify,
            ping,
        };

        let bootstrap_nodes = cfg.bootstrap_nodes.clone();

        let health = health_registry.map(|registry| {
            let bootstrap_nodes = BootstrapNodesHealthCheck::new(bootstrap_nodes);
            registry.register("bootstrap_nodes", bootstrap_nodes.clone());

            ConnectivityHealthChecks { bootstrap_nodes }
        });

        let connectivity = Connectivity {
            peer_id: cfg.local_peer_id,
            kademlia: kademlia_api,
            connection_pool: connection_pool_api,
            bootstrap_nodes: cfg.bootstrap_nodes.into_iter().collect(),
            bootstrap_frequency: cfg.bootstrap_frequency,
            metrics: cfg.connectivity_metrics,
            health,
        };

        (this, connectivity, particle_stream)
    }
}
