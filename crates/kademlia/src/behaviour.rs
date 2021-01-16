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

use libp2p::{kad::KademliaEvent, swarm::NetworkBehaviourEventProcess, PeerId};

use libp2p::kad;

use fluence_libp2p::types::OneshotOutlet;
use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::core::Multiaddr;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::QueryId;
use multihash::Multihash;
use particle_dht::DHTConfig;
use prometheus::Registry;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::task::Waker;
use trust_graph::TrustGraph;

#[derive(Debug)]
// TODO: impl error
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

#[derive(::libp2p::NetworkBehaviour)]
pub struct Kademlia {
    pub(super) kademlia: kad::Kademlia<MemoryStore>,

    #[behaviour(ignore)]
    pub(super) queries: HashMap<QueryId, QueryPromise>,
    #[behaviour(ignore)]
    pub(super) pending_peers: HashMap<PeerId, Vec<OneshotOutlet<Vec<Multiaddr>>>>,

    #[behaviour(ignore)]
    pub(super) waker: Option<Waker>,
}

impl Kademlia {
    pub fn new(config: DHTConfig, trust_graph: TrustGraph, registry: Option<&Registry>) -> Self {
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
            waker: None,
        }
    }
}

impl Kademlia {
    pub async fn bootstrap(&mut self) {}

    pub fn discover_peer(&mut self, peer: PeerId) -> BoxFuture<'static, Result<Vec<Multiaddr>>> {
        let (outlet, inlet) = oneshot::channel();

        self.kademlia.get_closest_peers(peer.clone());
        self.pending_peers.entry(peer).or_default().push(outlet);

        inlet
            .map(|r| r.map_err(|_| KademliaError::Cancelled))
            .boxed()
    }

    pub async fn neighborhood(&mut self, key: Multihash) -> Vec<PeerId> {
        todo!()
    }
}

impl Kademlia {
    pub(super) fn peer_discovered(&mut self, peer: &PeerId, maddrs: Vec<Multiaddr>) {
        println!("peer discovered {} {:?}", peer, maddrs);
        if let Some(outlets) = self.pending_peers.remove(peer) {
            for outlet in outlets {
                outlet.send(maddrs.clone()).ok();
            }
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Kademlia {
    fn inject_event(&mut self, event: KademliaEvent) {
        println!("kad event: {:?}", event);
        match event {
            KademliaEvent::QueryResult {
                id,
                result: kad::QueryResult::Bootstrap { .. },
                ..
            } => {}
            KademliaEvent::UnroutablePeer { .. } => {}
            KademliaEvent::RoutingUpdated {
                peer, addresses, ..
            } => {
                log::info!(target: "debug_kademlia", "routing updated {} {:#?}", peer, addresses);
                self.peer_discovered(&peer, addresses.into_vec())
            }
            KademliaEvent::RoutablePeer { peer, address }
            | KademliaEvent::PendingRoutablePeer { peer, address } => {
                self.peer_discovered(&peer, vec![address])
            }
            _ => {}
        }
    }
}

impl Deref for Kademlia {
    type Target = kad::Kademlia<MemoryStore>;

    fn deref(&self) -> &Self::Target {
        &self.kademlia
    }
}

impl DerefMut for Kademlia {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kademlia
    }
}

#[cfg(test)]
mod tests {
    use super::Kademlia;
    use crate::behaviour::KademliaError;
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
    use particle_dht::DHTConfig;
    use std::pin::Pin;
    use std::sync::mpsc;
    use std::task::Poll;
    use test_utils::{create_memory_maddr, enable_logs};
    use trust_graph::TrustGraph;

    fn dht_config() -> DHTConfig {
        let keypair = Keypair::generate();
        let public_key = Ed25519(keypair.public());
        let peer_id = PeerId::from(public_key);

        DHTConfig {
            peer_id,
            keypair,
            kad_config: Default::default(),
        }
    }

    fn make_node() -> (Swarm<Kademlia>, Multiaddr, PublicKey) {
        let trust_graph = TrustGraph::new(vec![]);
        let config = dht_config();
        let kp = libp2p::identity::Keypair::Ed25519(config.keypair.clone());
        let peer_id = config.peer_id.clone();
        let pk = config.keypair.public();
        let kad = Kademlia::new(config, trust_graph, None);

        let mut swarm = Swarm::new(build_memory_transport(kp), kad, peer_id);
        let maddr = create_memory_maddr();
        Swarm::listen_on(&mut swarm, maddr.clone());

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
        a.kademlia.bootstrap();

        // b knows only a, wants to discover c
        Swarm::dial_addr(&mut b, a_addr.clone()).unwrap();
        b.kademlia
            .add_address(Swarm::local_peer_id(&a), a_addr, a_pk);
        let discover_fut = b.discover_peer(Swarm::local_peer_id(&c).clone());

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
        assert_eq!(maddr.unwrap()[0], c_addr);
    }
}
