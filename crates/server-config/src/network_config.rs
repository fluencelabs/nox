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

use crate::{BootstrapConfig, KademliaConfig, ResolvedConfig};

use particle_protocol::ProtocolConfig;

use config_utils::to_peer_id;
use trust_graph::InMemoryStorage;

use libp2p::{core::Multiaddr, identity::Keypair, PeerId};
use prometheus::Registry;
use std::time::Duration;

type TrustGraph = trust_graph::TrustGraph<InMemoryStorage>;

pub struct NetworkConfig {
    pub key_pair: Keypair,
    pub local_peer_id: PeerId,
    pub node_version: &'static str,
    pub trust_graph: TrustGraph,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub bootstrap: BootstrapConfig,
    pub registry: Option<Registry>,
    pub protocol_config: ProtocolConfig,
    pub kademlia_config: KademliaConfig,
    pub particle_queue_buffer: usize,
    pub particle_parallelism: Option<usize>,
    pub bootstrap_frequency: usize,
    pub allow_local_addresses: bool,
    pub particle_timeout: Duration,
}

impl NetworkConfig {
    pub fn new(
        trust_graph: TrustGraph,
        registry: Option<Registry>,
        key_pair: Keypair,
        config: &ResolvedConfig,
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
            allow_local_addresses: config.allow_local_addresses,
            particle_timeout: config.particle_processing_timeout,
        }
    }
}
