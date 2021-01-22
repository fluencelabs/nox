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

use crate::error::{KademliaError, Result};

use control_macro::get_return;
use fluence_libp2p::types::OneshotOutlet;
use libp2p::core::Multiaddr;
use libp2p::identity::ed25519::Keypair;
use libp2p::kad;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{
    BootstrapError, BootstrapOk, BootstrapResult, GetClosestPeersError, GetClosestPeersOk,
    GetClosestPeersResult, QueryId, QueryResult,
};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{kad::KademliaEvent, swarm::NetworkBehaviourEventProcess, PeerId};
use multihash::Multihash;
use prometheus::Registry;
use std::collections::HashMap;
use trust_graph::TrustGraph;

pub struct KademliaConfig {
    pub peer_id: PeerId,
    pub keypair: Keypair,
    // TODO: wonderful name clashing. I guess it is better to rename one of the KademliaConfig's to something else. You'll figure it out.
    pub kad_config: server_config::KademliaConfig,
}

#[derive(Debug)]
pub enum PendingQuery {
    Peer(PeerId),
    Neighborhood(OneshotOutlet<Result<Vec<PeerId>>>),
    Unit(OneshotOutlet<Result<()>>),
}

#[derive(::libp2p::NetworkBehaviour)]
pub struct Kademlia {
    pub(super) kademlia: kad::Kademlia<MemoryStore>,

    #[behaviour(ignore)]
    pub(super) queries: HashMap<QueryId, PendingQuery>,
    #[behaviour(ignore)]
    pub(super) pending_peers: HashMap<PeerId, Vec<OneshotOutlet<Result<Vec<Multiaddr>>>>>,
}

impl Kademlia {
    pub fn new(
        config: KademliaConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> Self {
        let store = MemoryStore::new(config.peer_id.clone());

        let mut kademlia = kad::Kademlia::with_config(
            config.keypair.clone(),
            config.peer_id.clone(),
            store,
            config.kad_config.clone().into(),
            trust_graph,
        );

        if let Some(registry) = registry {
            kademlia.enable_metrics(registry);
        }

        Self {
            kademlia,
            queries: <_>::default(),
            pending_peers: <_>::default(),
        }
    }
}

impl Kademlia {
    pub fn bootstrap(&mut self, outlet: OneshotOutlet<Result<()>>) {
        if let Ok(query_id) = self.kademlia.bootstrap() {
            self.queries.insert(query_id, PendingQuery::Unit(outlet));
        } else {
            outlet.send(Err(KademliaError::NoKnownPeers)).ok();
        }
    }

    pub fn local_lookup(&mut self, peer: &PeerId, outlet: OneshotOutlet<Vec<Multiaddr>>) {
        outlet.send(self.kademlia.addresses_of_peer(peer)).ok();
    }

    pub fn discover_peer(&mut self, peer: PeerId, outlet: OneshotOutlet<Result<Vec<Multiaddr>>>) {
        let local = self.kademlia.addresses_of_peer(&peer);
        if !local.is_empty() {
            outlet.send(Ok(local)).ok();
            return;
        }
        let query_id = self.kademlia.get_closest_peers(peer);
        self.queries.insert(query_id, PendingQuery::Peer(peer));
        self.pending_peers.entry(peer).or_default().push(outlet);
    }

    pub fn neighborhood(&mut self, key: Multihash, outlet: OneshotOutlet<Result<Vec<PeerId>>>) {
        let query_id = self.kademlia.get_closest_peers(key);
        self.queries
            .insert(query_id, PendingQuery::Neighborhood(outlet));
    }
}

impl Kademlia {
    // TODO: timeout & non-discovered errors
    fn peer_discovered(&mut self, peer: PeerId, addresses: Vec<Multiaddr>) {
        if let Some(outlets) = self.pending_peers.remove(&peer) {
            for outlet in outlets {
                outlet.send(Ok(addresses.clone())).ok();
            }
        }
    }

    fn closest_finished(&mut self, id: QueryId, result: GetClosestPeersResult) {
        match get_return!(self.queries.remove(&id)) {
            PendingQuery::Peer(peer_id) => {
                let addresses = self.kademlia.addresses_of_peer(&peer_id);
                // if addresses are empty - do nothing, let it be finished by timeout
                // motivation: more addresses might appear later through other events
                if !addresses.is_empty() {
                    self.peer_discovered(peer_id, addresses)
                }
            }
            PendingQuery::Neighborhood(outlet) => {
                let neighbors = match result {
                    Ok(GetClosestPeersOk { peers, .. }) => peers,
                    Err(GetClosestPeersError::Timeout { peers, .. }) => peers,
                };
                let result = if neighbors.is_empty() {
                    Err(KademliaError::Timeout)
                } else {
                    Ok(neighbors)
                };
                outlet.send(result).ok();
            }
            PendingQuery::Unit(outlet) => {
                outlet.send(Ok(())).ok();
            }
        }
    }

    fn bootstrap_finished(&mut self, id: QueryId, result: BootstrapResult) {
        // how many buckets there are left to try
        let num_remaining = match result {
            Ok(BootstrapOk { num_remaining, .. }) => Some(num_remaining),
            Err(BootstrapError::Timeout { num_remaining, .. }) => num_remaining,
        };

        // if all desired buckets were tried, signal bootstrap completion
        // note that it doesn't care about successes or errors; that's because bootstrap keeps
        // going through next buckets even if it failed on previous buckets
        if num_remaining == Some(0) {
            if let Some(PendingQuery::Unit(outlet)) = self.queries.remove(&id) {
                outlet.send(Ok(())).ok();
            }
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Kademlia {
    fn inject_event(&mut self, event: KademliaEvent) {
        println!("kad event: {:?}", event);
        match event {
            KademliaEvent::QueryResult { id, result, .. } => match result {
                QueryResult::GetClosestPeers(result) => self.closest_finished(id, result),
                QueryResult::Bootstrap(result) => self.bootstrap_finished(id, result),
                QueryResult::GetProviders(_) => {}
                QueryResult::StartProviding(_) => {}
                QueryResult::RepublishProvider(_) => {}
                QueryResult::GetRecord(_) => {}
                QueryResult::PutRecord(_) => {}
                QueryResult::RepublishRecord(_) => {}
            },
            KademliaEvent::UnroutablePeer { .. } => {}
            KademliaEvent::RoutingUpdated {
                peer, addresses, ..
            } => {
                log::info!(target: "debug_kademlia", "routing updated {} {:#?}", peer, addresses);
                self.peer_discovered(peer, addresses.into_vec())
            }
            KademliaEvent::RoutablePeer { peer, address }
            | KademliaEvent::PendingRoutablePeer { peer, address } => {
                self.peer_discovered(peer, vec![address])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kademlia;
    use crate::behaviour::KademliaError;
    use crate::KademliaConfig;
    use async_std::stream::IntoStream;
    use async_std::task;
    use fluence_libp2p::build_memory_transport;
    use futures::channel::oneshot;
    use futures::future::Fuse;
    use futures::ready;
    use futures::select;
    use futures::FutureExt;
    use futures::StreamExt;
    use libp2p::core::Multiaddr;
    use libp2p::identity::ed25519::{Keypair, PublicKey};
    use libp2p::identity::PublicKey::Ed25519;
    use libp2p::swarm::SwarmEvent;
    use libp2p::PeerId;
    use libp2p::Swarm;
    use std::pin::Pin;
    use std::sync::mpsc;
    use std::task::Poll;
    use test_utils::{create_memory_maddr, enable_logs};
    use trust_graph::TrustGraph;

    fn kad_config() -> KademliaConfig {
        let keypair = Keypair::generate();
        let public_key = Ed25519(keypair.public());
        let peer_id = PeerId::from(public_key);

        KademliaConfig {
            peer_id,
            keypair,
            kad_config: Default::default(),
        }
    }

    fn make_node() -> (Swarm<Kademlia>, Multiaddr, PublicKey) {
        let trust_graph = TrustGraph::new(vec![]);
        let config = kad_config();
        let kp = libp2p::identity::Keypair::Ed25519(config.keypair.clone());
        let peer_id = config.peer_id.clone();
        let pk = config.keypair.public();
        let kad = Kademlia::new(config, trust_graph, None);

        let mut swarm = Swarm::new(build_memory_transport(kp), kad, peer_id);
        let maddr = create_memory_maddr();
        Swarm::listen_on(&mut swarm, maddr.clone()).ok();

        (swarm, maddr, pk)
    }

    #[test]
    fn discovery() {
        enable_logs();

        let (mut a, a_addr, a_pk) = make_node();
        let (mut b, b_addr, b_pk) = make_node();
        let (c, c_addr, c_pk) = make_node();
        let (d, d_addr, d_pk) = make_node();
        let (e, e_addr, e_pk) = make_node();

        // a knows everybody
        Swarm::dial_addr(&mut a, b_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, c_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, d_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, e_addr.clone()).unwrap();
        a.kademlia
            .add_address(Swarm::local_peer_id(&b), b_addr.clone(), b_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&c), c_addr.clone(), c_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&d), d_addr.clone(), d_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&e), e_addr.clone(), e_pk);
        a.kademlia.bootstrap().ok();

        // b knows only a, wants to discover c
        Swarm::dial_addr(&mut b, a_addr.clone()).unwrap();
        b.kademlia
            .add_address(Swarm::local_peer_id(&a), a_addr, a_pk);
        let (out, inlet) = oneshot::channel();
        b.discover_peer(Swarm::local_peer_id(&c).clone(), out);
        let discover_fut = inlet;

        let maddr = async_std::task::block_on(async move {
            let mut swarms = vec![a, b, c, d, e];
            task::spawn(futures::future::poll_fn(move |ctx| {
                for (i, swarm) in swarms.iter_mut().enumerate() {
                    if let Poll::Ready(Some(e)) = swarm.poll_next_unpin(ctx) {
                        println!("event: {:#?}", e);
                        return Poll::Ready(());
                    }
                }
                Poll::Pending
            }));

            discover_fut.await
        });

        println!("maddr: {:?}", maddr);
        assert_eq!(maddr.unwrap().unwrap().1[0], c_addr);
    }
}
