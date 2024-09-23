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

use libp2p::{core::Multiaddr, identity::Keypair, PeerId};
use libp2p_connection_limits::ConnectionLimits;
use libp2p_metrics::Metrics;
use std::sync::Arc;
use std::time::Duration;

use config_utils::to_peer_id;
use particle_protocol::ProtocolConfig;

use crate::kademlia_config::KademliaConfig;
use crate::{BootstrapConfig, ResolvedConfig};

pub struct NetworkConfig {
    pub key_pair: Keypair,
    pub local_peer_id: PeerId,
    pub node_version: &'static str,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub bootstrap: BootstrapConfig,
    pub libp2p_metrics: Option<Arc<Metrics>>,
    pub protocol_config: ProtocolConfig,
    pub kademlia_config: KademliaConfig,
    pub particle_queue_buffer: usize,
    pub bootstrap_frequency: usize,
    pub connection_limits: ConnectionLimits,
    pub connection_idle_timeout: Duration,
}

impl NetworkConfig {
    pub fn new(
        libp2p_metrics: Option<Arc<Metrics>>,
        key_pair: Keypair,
        config: &ResolvedConfig,
        node_version: &'static str,
        connection_limits: ConnectionLimits,
    ) -> Self {
        Self {
            node_version,
            libp2p_metrics,
            local_peer_id: to_peer_id(&key_pair),
            key_pair,
            bootstrap_nodes: config.bootstrap_nodes.clone(),
            bootstrap: config.bootstrap_config.clone(),
            protocol_config: config.protocol_config.clone(),
            kademlia_config: config.kademlia.clone(),
            particle_queue_buffer: config.particle_queue_buffer,
            bootstrap_frequency: config.bootstrap_frequency,
            connection_limits,
            connection_idle_timeout: config.node_config.transport_config.connection_idle_timeout,
        }
    }
}
