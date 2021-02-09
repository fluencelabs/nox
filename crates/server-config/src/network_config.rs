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

use crate::NodeConfig;
use crate::{BootstrapConfig, KademliaConfig};

use particle_protocol::ProtocolConfig;

use config_utils::to_peer_id;
use trust_graph::TrustGraph;

use libp2p::{core::Multiaddr, identity::ed25519, PeerId};
use prometheus::Registry;

pub struct NetworkConfig {
    pub key_pair: ed25519::Keypair,
    pub local_peer_id: PeerId,
    pub trust_graph: TrustGraph,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub bootstrap: BootstrapConfig,
    pub registry: Option<Registry>,
    pub protocol_config: ProtocolConfig,
    pub kademlia_config: KademliaConfig,
    pub particle_queue_buffer: usize,
    pub particle_parallelism: usize,
    pub bootstrap_frequency: usize,
}

impl NetworkConfig {
    pub fn new(
        trust_graph: TrustGraph,
        registry: Option<Registry>,
        key_pair: ed25519::Keypair,
        config: &NodeConfig,
    ) -> Self {
        Self {
            trust_graph,
            registry,
            local_peer_id: to_peer_id(&key_pair),
            key_pair,
            bootstrap_nodes: config.bootstrap_nodes.clone(),
            bootstrap: config.bootstrap_config.clone(),
            protocol_config: config.protocol_config.clone(),
            kademlia_config: config.kademlia.clone(),
            particle_queue_buffer: config.particle_queue_buffer,
            particle_parallelism: config.particle_processor_parallelism,
            bootstrap_frequency: config.bootstrap_frequency,
        }
    }
}
