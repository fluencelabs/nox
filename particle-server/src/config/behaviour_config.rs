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

use crate::config::ServerConfig;
use crate::BootstrapConfig;

use particle_protocol::ProtocolConfig;

use config_utils::to_peer_id;
use trust_graph::TrustGraph;

use libp2p::{core::Multiaddr, identity::ed25519, PeerId};
use prometheus::Registry;
use std::{collections::HashMap, path::PathBuf};

pub struct BehaviourConfig<'a> {
    pub key_pair: ed25519::Keypair,
    pub local_peer_id: PeerId,
    pub listening_addresses: Vec<Multiaddr>,
    pub trust_graph: TrustGraph,
    pub bootstrap_nodes: Vec<Multiaddr>,
    pub bootstrap: BootstrapConfig,
    pub registry: Option<&'a Registry>,
    pub services_base_dir: PathBuf,
    pub services_envs: HashMap<Vec<u8>, Vec<u8>>,
    pub stepper_base_dir: PathBuf,
    pub protocol_config: ProtocolConfig,
    pub stepper_pool_size: usize,
}

impl<'a> BehaviourConfig<'a> {
    pub fn new(
        trust_graph: TrustGraph,
        registry: Option<&'a Registry>,
        key_pair: ed25519::Keypair,
        config: &ServerConfig,
    ) -> Self {
        Self {
            trust_graph,
            registry,
            local_peer_id: to_peer_id(&key_pair),
            key_pair,
            listening_addresses: config.external_addresses(),
            bootstrap_nodes: config.bootstrap_nodes.clone(),
            bootstrap: config.bootstrap_config.clone(),
            services_base_dir: config.services_base_dir.clone(),
            services_envs: config.services_envs.clone(),
            stepper_base_dir: config.stepper_base_dir.clone(),
            protocol_config: config.protocol_config.clone(),
            stepper_pool_size: config.stepper_pool_size,
        }
    }
}
