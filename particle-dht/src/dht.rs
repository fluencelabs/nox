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

use crate::store::memory_store::MemoryStore;
use libp2p::{
    identity::ed25519::Keypair,
    kad::{Kademlia, KademliaConfig, KademliaEvent},
    swarm::NetworkBehaviourEventProcess,
    PeerId,
};
use prometheus::Registry;
use std::time::Duration;
use trust_graph::TrustGraph;

#[derive(libp2p::NetworkBehaviour)]
pub struct ParticleDHT {
    pub(super) kademlia: Kademlia<MemoryStore>,
}

pub struct DHTConfig {
    pub peer_id: PeerId,
    pub keypair: Keypair,
}

impl ParticleDHT {
    pub fn new(config: DHTConfig, trust_graph: TrustGraph, registry: Option<&Registry>) -> Self {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5))
            .set_max_packet_size(100 * 4096 * 4096) // 100 Mb
            .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap())
            .set_connection_idle_timeout(Duration::from_secs(2_628_000_000)); // ~month
        let store = MemoryStore::new(config.peer_id.clone());

        let mut kademlia = Kademlia::with_config(
            config.keypair.clone(),
            config.peer_id.clone(),
            store,
            cfg,
            trust_graph,
        );
        /*
        if let Some(registry) = registry {
            kademlia.enable_metrics(registry);
        }
        */
        Self { kademlia }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for ParticleDHT {
    fn inject_event(&mut self, event: KademliaEvent) {
        unimplemented!()
    }
}
