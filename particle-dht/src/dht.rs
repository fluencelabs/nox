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

use control_macro::get_return;
use fluence_libp2p::generate_swarm_event_type;
use libp2p::kad::{PutRecordError, PutRecordResult, QueryResult};
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::{
    identity::ed25519::Keypair,
    kad::{
        store::{self, MemoryStore},
        Kademlia, KademliaConfig, KademliaEvent, QueryId, Quorum, Record,
    },
    swarm::NetworkBehaviourEventProcess,
    PeerId,
};
use prometheus::Registry;
use smallvec::alloc::collections::VecDeque;
use std::collections::HashMap;
use std::time::Duration;
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(ParticleDHT);

#[derive(Debug)]
pub enum PublishError {
    StoreError(store::Error),
    TimedOut,
    QuorumFailed,
}

#[derive(Debug)]
pub enum DHTEvent {
    Published(PeerId),
    PublishFailed(PeerId, PublishError),
}

pub struct ParticleDHT {
    pub(super) kademlia: Kademlia<MemoryStore>,
    pub(super) config: DHTConfig,
    pub(super) pending: HashMap<QueryId, PeerId>,
    pub(super) events: VecDeque<SwarmEventType>,
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

        if let Some(registry) = registry {
            kademlia.enable_metrics(registry);
        }

        Self {
            kademlia,
            config,
            pending: <_>::default(),
            events: <_>::default(),
        }
    }
}

impl ParticleDHT {
    pub fn publish_client(&mut self, client: PeerId) {
        let bytes = [client.as_bytes(), self.config.peer_id.as_bytes()].concat();
        let signature = self.config.keypair.sign(bytes.as_slice());
        let record = Record::new(client.clone().into_bytes(), signature);

        match self.kademlia.put_record(record, Quorum::Majority) {
            Ok(query_id) => {
                self.pending.insert(query_id, client);
            }
            Err(err) => self.publish_failed(client, PublishError::StoreError(err)),
        };
    }

    pub fn publish_failed(&mut self, client: PeerId, error: PublishError) {
        self.emit(DHTEvent::PublishFailed(client, error));
    }

    pub fn publish_succeeded(&mut self, client: PeerId) {
        self.emit(DHTEvent::Published(client))
    }

    fn emit(&mut self, event: DHTEvent) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));
    }

    pub(super) fn recover_result(result: PutRecordResult) -> Result<(), PublishError> {
        let err = match result {
            Ok(_) => return Ok(()),
            Err(err) => err,
        };

        #[rustfmt::skip]
        let (found, quorum, reason, key) = match &err {
            PutRecordError::QuorumFailed { success, quorum, key, .. } => (success.len(), quorum.get(), PublishError::QuorumFailed, key),
            PutRecordError::Timeout { success, quorum, key, .. } => (success.len(), quorum.get(), PublishError::TimedOut, key),
        };

        // TODO: is 50% a reasonable number?
        // TODO: move to config
        // Recover if found more than 50% of required quorum
        if found * 2 > quorum {
            #[rustfmt::skip]
            log::warn!("DHT.put almost failed, saved {} of {} replicas, but it is good enough", found, quorum);
            return Ok(());
        }

        let err_msg = format!(
            "Error while publishing provider for {:?}: DHT.put failed ({}/{} replies): {:?}",
            key, found, quorum, reason
        );
        log::warn!("{}", err_msg);

        Err(reason)
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for ParticleDHT {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::QueryResult {
                id,
                result: QueryResult::PutRecord(result),
                ..
            } => {
                let client = get_return!(self.pending.remove(&id));
                if let Err(err) = Self::recover_result(result) {
                    self.publish_failed(client, err)
                } else {
                    self.publish_succeeded(client)
                }
            }
            KademliaEvent::QueryResult { .. } => {}
            KademliaEvent::RoutingUpdated { .. } => {}
            KademliaEvent::UnroutablePeer { .. } => {}
            KademliaEvent::RoutablePeer { .. } => {}
            KademliaEvent::PendingRoutablePeer { .. } => {}
        }
    }
}
