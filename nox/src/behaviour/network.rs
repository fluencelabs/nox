/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use libp2p::identify::Config as IdentifyConfig;
use libp2p::{
    connection_limits::Behaviour as ConnectionLimits,
    identify::Behaviour as Identify,
    ping::{Behaviour as Ping, Config as PingConfig},
    swarm::NetworkBehaviour,
    PeerId,
};
use tokio::sync::mpsc;

use connection_pool::ConnectionPoolBehaviour;
use health::HealthCheckRegistry;
use kademlia::{Kademlia, KademliaConfig};
use particle_protocol::{ExtendedParticle, PROTOCOL_NAME};
use peer_metrics::{ConnectionPoolMetrics, ConnectivityMetrics};
use server_config::NetworkConfig;

use crate::connectivity::Connectivity;
use crate::health::{BootstrapNodesHealth, ConnectivityHealth, KademliaBootstrapHealth};

/// Coordinates protocols, so they can cooperate
#[derive(NetworkBehaviour)]
pub struct FluenceNetworkBehaviour {
    identify: Identify,
    ping: Ping,
    connection_limits: ConnectionLimits,
    pub(crate) connection_pool: ConnectionPoolBehaviour,
    pub(crate) kademlia: Kademlia,
}

struct KademliaConfigAdapter {
    peer_id: PeerId,
    config: server_config::KademliaConfig,
}

impl From<KademliaConfigAdapter> for KademliaConfig {
    fn from(value: KademliaConfigAdapter) -> Self {
        Self {
            peer_id: value.peer_id,
            max_packet_size: value.config.max_packet_size,
            query_timeout: value.config.query_timeout,
            replication_factor: value.config.replication_factor,
            peer_fail_threshold: value.config.peer_fail_threshold,
            ban_cooldown: value.config.ban_cooldown,
            protocol_name: value.config.protocol_name,
        }
    }
}

impl FluenceNetworkBehaviour {
    pub fn new(
        cfg: NetworkConfig,
        health_registry: Option<&mut HealthCheckRegistry>,
        connectivity_metrics: Option<ConnectivityMetrics>,
        connection_pool_metrics: Option<ConnectionPoolMetrics>,
    ) -> (Self, Connectivity, mpsc::Receiver<ExtendedParticle>) {
        let local_public_key = cfg.key_pair.public();
        let identify = Identify::new(
            IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key)
                .with_agent_version(cfg.node_version.into()),
        );
        let ping = Ping::new(PingConfig::new());

        let kad_config = KademliaConfigAdapter {
            peer_id: cfg.local_peer_id,
            config: cfg.kademlia_config,
        };

        let (kademlia, kademlia_api) = Kademlia::new(kad_config.into(), cfg.libp2p_metrics);
        let (connection_pool, particle_stream, connection_pool_api) = ConnectionPoolBehaviour::new(
            cfg.particle_queue_buffer,
            cfg.protocol_config,
            cfg.local_peer_id,
            connection_pool_metrics,
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
            let bootstrap_nodes = BootstrapNodesHealth::new(bootstrap_nodes);
            let kademlia_bootstrap = KademliaBootstrapHealth::default();
            registry.register("bootstrap_nodes", bootstrap_nodes.clone());
            registry.register("kademlia_bootstrap", kademlia_bootstrap.clone());

            ConnectivityHealth {
                bootstrap_nodes,
                kademlia_bootstrap,
            }
        });

        let connectivity = Connectivity {
            peer_id: cfg.local_peer_id,
            kademlia: kademlia_api,
            connection_pool: connection_pool_api,
            bootstrap_nodes: cfg.bootstrap_nodes.into_iter().collect(),
            bootstrap_frequency: cfg.bootstrap_frequency,
            metrics: connectivity_metrics,
            health,
        };

        (this, connectivity, particle_stream)
    }
}
