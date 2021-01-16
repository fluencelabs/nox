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
    pub(super) pending_peers: HashMap<PeerId, Vec<OneshotOutlet<Multiaddr>>>,

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

    pub fn discover_peer(&mut self, peer: PeerId) -> BoxFuture<'static, Result<Multiaddr>> {
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
    pub(super) fn peer_discovered(&mut self, peer: &PeerId, maddr: Multiaddr) {
        if let Some(outlets) = self.pending_peers.remove(peer) {
            for outlet in outlets {
                outlet.send(maddr.clone()).ok();
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
            r @ KademliaEvent::RoutingUpdated { .. } => {
                log::info!(target: "debug_kademlia", "routing updated {:#?}", r)
            }
            KademliaEvent::RoutablePeer { peer, address }
            | KademliaEvent::PendingRoutablePeer { peer, address } => {
                self.peer_discovered(&peer, address)
            }
            _ => {}
        }
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
    use futures::select;
    use futures::FutureExt;
    use futures::StreamExt;
    use libp2p::core::Multiaddr;
    use libp2p::identity::ed25519::Keypair;
    use libp2p::identity::PublicKey::Ed25519;
    use libp2p::swarm::SwarmEvent;
    use libp2p::PeerId;
    use libp2p::Swarm;
    use particle_dht::DHTConfig;
    use std::pin::Pin;
    use std::sync::mpsc;
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

    fn make_node() -> (Swarm<Kademlia>, Multiaddr) {
        let trust_graph = TrustGraph::new(vec![]);
        let config = dht_config();
        let kp = libp2p::identity::Keypair::Ed25519(config.keypair.clone());
        let peer_id = config.peer_id.clone();
        let kad = Kademlia::new(config, trust_graph, None);

        let mut swarm = Swarm::new(build_memory_transport(kp), kad, peer_id);
        let maddr = create_memory_maddr();
        Swarm::listen_on(&mut swarm, maddr.clone());

        (swarm, maddr)
    }

    #[test]
    fn discovery() {
        enable_logs();

        let (mut a, a_addr) = make_node();
        let (mut b, b_addr) = make_node();
        let (c, c_addr) = make_node();
        let (d, d_addr) = make_node();
        let (e, e_addr) = make_node();

        // a knows everybody
        Swarm::dial_addr(&mut a, b_addr).unwrap();
        Swarm::dial_addr(&mut a, c_addr).unwrap();

        // b knows only a, wants to discover c
        Swarm::dial_addr(&mut b, a_addr).unwrap();

        let maddr = async_std::task::block_on(async move {
            let mut a = a;
            let mut b = b;
            let mut c = c;
            let mut d = d;
            let mut e = e;

            // let b_thread = task::spawn(async move {
            //     loop {
            //         match b.next_event().await {
            //             SwarmEvent::ConnectionEstablished { .. } => {
            //                 let maddr = b.discover_peer(Swarm::local_peer_id(&c).clone()).await;
            //                 break maddr;
            //             }
            //             _ => {}
            //         }
            //     }
            // });

            let c_peer_id = Swarm::local_peer_id(&c).clone();
            task::spawn(async move {
                loop {
                    select!(
                        _ = a.select_next_some() => {},
                        _ = c.select_next_some() => {},
                        _ = d.select_next_some() => {},
                        _ = e.select_next_some() => {},
                    )
                }
            });

            println!("starting loop");

            let (outlet, inlet) = async_std::channel::unbounded();
            task::spawn(async move {
                loop {
                    let outlet = outlet.clone();
                    match b.next_event().await {
                        SwarmEvent::ConnectionEstablished { .. } => {
                            println!("connection established, discovering");

                            let promise = b.discover_peer(c_peer_id.clone());
                            task::spawn(async move {
                                let maddr = promise.await;
                                outlet.clone().send(maddr).await.ok();
                            });
                        }
                        e => {
                            println!("swarm event: {:#?}", e);
                        }
                    }
                }
            });

            inlet.recv().await;
        });

        println!("{:?}", maddr);
    }
}
