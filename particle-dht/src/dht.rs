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

use super::wait_peer::WaitPeer;

use trust_graph::TrustGraph;
use waiting_queues::WaitingQueues;

use libp2p::swarm::DialPeerCondition;
use libp2p::{
    core::{identity::ed25519, Multiaddr},
    identity::ed25519::Keypair,
    kad::{
        store::{self, MemoryStore},
        Kademlia, KademliaConfig, QueryId,
    },
    PeerId,
};
use particle_protocol::Particle;
use prometheus::Registry;
use smallvec::alloc::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::task::{Context, Poll, Waker};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[derive(Debug)]
pub enum PublishError {
    StoreError(store::Error),
    TimedOut,
    QuorumFailed,
}

#[derive(Debug)]
pub enum DHTEvent {
    DialPeer {
        peer_id: PeerId,
        condition: DialPeerCondition,
    },
    Published(PeerId),
    PublishFailed(PeerId, PublishError),
    Forward {
        target: PeerId,
        particle: Particle,
    },
}

pub struct ParticleDHT {
    pub(super) kademlia: Kademlia<MemoryStore>,
    pub(super) config: DHTConfig,
    pub(super) pending: HashMap<QueryId, PeerId>,
    pub(super) events: VecDeque<DHTEvent>,
    pub(super) connected_peers: HashSet<PeerId>,
    pub(super) wait_peer: WaitingQueues<PeerId, WaitPeer>,
    pub(super) waker: Option<Waker>,
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
            connected_peers: <_>::default(),
            wait_peer: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<DHTEvent> {
        self.waker = Some(cx.waker().clone());

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }

    pub fn add_kad_node(
        &mut self,
        node_id: PeerId,
        addresses: Vec<Multiaddr>,
        public_key: ed25519::PublicKey,
    ) {
        log::trace!(
            "adding new node {} with {:?} addresses to kademlia",
            node_id,
            addresses,
        );
        for addr in addresses {
            self.kademlia
                .add_address(&node_id, addr.clone(), public_key.clone());
        }
    }

    /// Run kademlia bootstrap, to advertise ourselves in Kademlia
    pub fn bootstrap(&mut self) {
        log::info!("Bootstrapping");
        use std::borrow::Borrow;
        // NOTE: Using Qm form of `peer_id` here (via peer_id.borrow), since kademlia uses that for keys
        self.kademlia
            .get_closest_peers(self.config.peer_id.borrow());
    }

    pub(super) fn bootstrap_finished(&mut self) {}

    pub(super) fn push_event(&mut self, event: DHTEvent) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }

        self.events.push_back(event);
    }
}

impl Deref for ParticleDHT {
    type Target = Kademlia<MemoryStore>;

    fn deref(&self) -> &Self::Target {
        &self.kademlia
    }
}

impl DerefMut for ParticleDHT {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kademlia
    }
}
