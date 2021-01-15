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

use libp2p::{
    kad::{GetClosestPeersError, GetClosestPeersOk, KademliaEvent},
    swarm::NetworkBehaviourEventProcess,
    PeerId,
};

use libp2p::kad;

use crate::behaviour::QueryResult::Peer;
use fluence_libp2p::types::OneshotOutlet;
use futures::channel::oneshot;
use futures::channel::oneshot::Canceled;
use futures::{FutureExt, SinkExt};
use libp2p::core::Multiaddr;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::QueryId;
use multihash::Multihash;
use std::collections::HashMap;
use std::task::Waker;

pub enum KademliaError {
    Timeout,
    Cancelled,
}

#[derive(Debug)]
pub enum QueryPromise {
    Neighborhood(OneshotOutlet<Vec<PeerId>>),
    Unit(OneshotOutlet<()>),
}

type Result<T> = std::result::Result<T, KademliaError>;

pub struct Kademlia {
    pub(super) kademlia: kad::Kademlia<MemoryStore>,

    pub(super) queries: HashMap<QueryId, QueryPromise>,
    pub(super) pending_peers: HashMap<PeerId, Vec<OneshotOutlet<Multiaddr>>>,

    pub(super) waker: Option<Waker>,
}

impl Kademlia {
    async fn bootstrap(&mut self) {}

    async fn discover_peer(&mut self, peer: PeerId) -> Result<Multiaddr> {
        let (outlet, inlet) = oneshot::channel();

        self.kademlia.get_closest_peers(peer.clone());
        self.pending_peers.entry(peer).or_default().push(outlet);

        inlet
            .map(r.map_err(|_| Err(KademliaError::Cancelled)))
            .await
    }

    async fn neighborhood(&mut self, key: Multihash) -> Vec<PeerId> {
        todo!()
    }
}

impl Kademlia {
    pub(super) fn peer_discovered(&mut self, peer: &PeerId, maddr: Multiaddr) {
        if let Some(outlets) = self.pending_peers.remove(peer) {
            for mut outlet in outlets {
                outlet.send(maddr.clone())
            }
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Kademlia {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::QueryResult {
                id,
                result: kad::QueryResult::Bootstrap { .. },
                ..
            } => {}
            KademliaEvent::UnroutablePeer { .. } => {}
            r @ KademliaEvent::RoutingUpdated { .. } => {
                log::info!(target: "debug_kademlia", "routing updated {:#?}", r)
            }
            KademliaEvent::RoutablePeer { peer, address }
            | KademliaEvent::PendingRoutablePeer { peer, address } => {
                self.peer_discovered(&peer, address)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn discovery() {}
}
