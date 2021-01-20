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

use crate::connection_pool::{ConnectionPool, Contact};

use fluence_libp2p::types::{
    BackPressuredInlet, BackPressuredOutlet, OneshotInlet, OneshotOutlet, Outlet,
};
use fluence_libp2p::{generate_swarm_event_type, remote_multiaddr};
use particle_protocol::{CompletionChannel, HandlerMessage, Particle, ProtocolConfig};

use std::collections::{HashMap, HashSet, VecDeque};
use std::task::{Context, Poll, Waker};

use futures::channel::mpsc::SendError;
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::ready;
use futures::FutureExt;
use futures::{future, SinkExt};
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput::{First, Second};
use libp2p::core::{ConnectedPoint, Multiaddr};
use libp2p::identity::ed25519::Keypair;
use libp2p::identity::PublicKey::Ed25519;
use libp2p::kad::Kademlia;
use libp2p::swarm::{
    DialPeerCondition, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
    NetworkBehaviourEventProcess, NotifyHandler, OneShotHandler, PollParameters, ProtocolsHandler,
};
use libp2p::PeerId;
use std::collections::hash_map::Entry;
use std::hint::unreachable_unchecked;
use trust_graph::TrustGraph;

type SwarmEventType = generate_swarm_event_type!(ConnectionPoolBehaviour);

#[derive(Debug)]
enum Peer {
    Connected(HashSet<Multiaddr>),
    // TODO: not sure if neet to store multiaddrs in dialing
    Dialing(HashSet<Multiaddr>, Vec<OneshotOutlet<bool>>),
}

impl Peer {
    fn addresses(&self) -> impl Iterator<Item = &Multiaddr> {
        match self {
            Peer::Connected(addrs) => addrs.iter(),
            Peer::Dialing(addrs, _) => addrs.iter(),
        }
    }
}

pub struct ConnectionPoolBehaviour {
    outlet: BackPressuredOutlet<Particle>,
    queue: VecDeque<Particle>,

    contacts: HashMap<PeerId, Peer>,

    events: VecDeque<SwarmEventType>,
    waker: Option<Waker>,
    protocol_config: ProtocolConfig,
}

impl ConnectionPool for ConnectionPoolBehaviour {
    fn connect(&mut self, contact: Contact) -> BoxFuture<'static, bool> {
        let (outlet, inlet) = futures::channel::oneshot::channel();
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id: contact.peer_id.clone(),
            condition: DialPeerCondition::Always,
        });

        match self.contacts.entry(contact.peer_id.clone()) {
            Entry::Occupied(mut entry) => match entry.get_mut() {
                // TODO: add/replace multiaddr? if yes, do not forget to check connectivity
                Peer::Connected(_) => {
                    outlet.send(true).ok();
                }
                Peer::Dialing(addrs, outlets) => {
                    if let Some(maddr) = contact.addr {
                        addrs.insert(maddr);
                    }
                    outlets.push(outlet)
                }
            },
            Entry::Vacant(slot) => {
                slot.insert(Peer::Dialing(
                    contact.addr.into_iter().collect(),
                    vec![outlet],
                ));
            }
        }

        let peer_id = contact.peer_id;
        inlet
            .map(move |r| {
                r.map(|_| true).unwrap_or_else(|err| {
                    log::warn!("error connecting to {}, oneshot cancelled", peer_id);
                    false
                })
            })
            .boxed()
    }

    fn disconnect(&mut self, contact: Contact) -> BoxFuture<'static, bool> {
        todo!(
            "this doesn't make sense with OneShotHandler since connections are short-lived {:?}",
            contact
        )
    }

    fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.contacts.contains_key(peer_id)
    }

    fn get_contact(&self, peer_id: &PeerId) -> Option<Contact> {
        match self.contacts.get(peer_id) {
            Some(Peer::Connected(addrs)) => Some(Contact {
                peer_id: peer_id.clone(),
                addr: addrs.iter().next().cloned(),
            }),
            _ => None,
        }
    }

    fn send(&mut self, to: Contact, particle: Particle) -> BoxFuture<'static, bool> {
        let (outlet, inlet) = oneshot::channel();

        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: to.peer_id,
                handler: NotifyHandler::Any,
                event: HandlerMessage::OutParticle(particle, CompletionChannel::Oneshot(outlet)),
            });

        inlet.map(|r| r.is_ok()).boxed()
    }
}

impl ConnectionPoolBehaviour {
    pub fn new(
        buffer: usize,
        protocol_config: ProtocolConfig,
    ) -> (Self, BackPressuredInlet<Particle>) {
        let (outlet, inlet) = mpsc::channel(buffer);

        let this = Self {
            outlet,
            queue: <_>::default(),
            contacts: <_>::default(),
            events: <_>::default(),
            waker: None,
            protocol_config,
        };

        (this, inlet)
    }

    pub fn kad_discover(&mut self, peer_id: PeerId) -> BoxFuture<'static, Contact> {
        futures::future::ready(Contact {
            peer_id,
            addr: None,
        })
        .boxed()
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

impl NetworkBehaviour for ConnectionPoolBehaviour {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.protocol_config.clone().into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.contacts
            .get(peer_id)
            .into_iter()
            .flat_map(|p| p.addresses().cloned())
            .collect()
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.contacts.remove(peer_id);
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let multiaddr = remote_multiaddr(cp).clone();
        match self.contacts.entry(peer_id.clone()) {
            Entry::Occupied(mut entry) => {
                match entry.get_mut() {
                    Peer::Connected(addrs) => {
                        addrs.insert(multiaddr);
                    }
                    Peer::Dialing(..) => {
                        let mut set = HashSet::new();
                        set.insert(multiaddr);
                        let value = entry.insert(Peer::Connected(set));
                        if let Peer::Dialing(_, outlets) = value {
                            for outlet in outlets {
                                outlet.send(true).ok();
                            }
                        }
                    }
                };
            }
            Entry::Vacant(e) => {
                let mut set = HashSet::new();
                set.insert(multiaddr);
                e.insert(Peer::Connected(set));
            }
        }
    }

    fn inject_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            HandlerMessage::InParticle(particle) => {
                self.queue.push_back(particle);
                self.wake();
            }
            HandlerMessage::InboundUpgradeError(err) => log::warn!("UpgradeError: {:?}", err),
            HandlerMessage::Upgrade => {}
            HandlerMessage::OutParticle(..) => unreachable!("can't receive OutParticle"),
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>, _: &mut impl PollParameters) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        loop {
            // Check backpressure on the outlet
            match self.outlet.poll_ready(cx) {
                Poll::Ready(Ok(_)) => {
                    // channel is ready to consume more particles, so send them
                    if let Some(particle) = self.queue.pop_front() {
                        self.outlet.start_send(particle).ok();
                    } else {
                        break;
                    }
                }
                Poll::Pending => {
                    // if channel is full, then keep particles in the queue
                    if self.queue.len() > 100 {
                        log::warn!("Particle queue seems to have stalled");
                    }
                    break;
                }
                Poll::Ready(Err(err)) => {
                    log::warn!("ConnectionPool particle inlet has been dropped: {}", err);
                    break;
                }
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use crate::behaviour::ConnectionPoolBehaviour;
    use crate::connection_pool::ConnectionPool;

    use async_std::sync::Mutex;
    use async_std::task;
    use fluence_libp2p::{build_memory_transport, RandomPeerId};
    use futures::future::BoxFuture;
    use futures::task::{Context, Poll};
    use futures::StreamExt;
    use futures::{select, Stream};
    use futures::{Future, FutureExt};
    use libp2p::core::connection::ConnectionId;
    use libp2p::identity::ed25519::Keypair;
    use libp2p::identity::PublicKey::Ed25519;
    use libp2p::swarm::{ExpandedSwarm, NetworkBehaviour};
    use libp2p::{identity, PeerId, Swarm};
    use particle_dht::DHTConfig;
    use particle_protocol::{HandlerMessage, Particle};
    use std::ops::{Deref, DerefMut};
    use std::pin::Pin;
    use std::sync::Arc;
    use trust_graph::TrustGraph;

    fn fce_exec(particle: Particle) -> BoxFuture<'static, (Vec<PeerId>, Particle)> {
        futures::future::ready((vec![particle.init_peer_id.clone()], particle)).boxed()
    }

    macro_rules! unlock (
        ($lock:ident.$method:ident($($args:expr),*)$(.$await:ident)?) => (
            {
                #[allow(unused_mut)]
                let mut guard = $lock.lock().await;
                let result = guard.$method($($args),*);
                drop(guard);
                $(let result = result.$await;)?
                result
            }
        );
    );

    async fn lockF<T, R, F: Future<Output = R>>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> F) -> R {
        let mut guard = m.lock().await;
        let result = f(guard.deref_mut());
        drop(guard);
        result.await
    }

    async fn lock<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = m.lock().await;
        let result = f(guard.deref_mut());
        drop(guard);
        result
    }

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

    #[test]
    fn run() {
        let spawned = task::spawn(async move {
            let (node, particles) =
                ConnectionPoolBehaviour::new(100, dht_config(), TrustGraph::new(vec![]));
            let keypair = Keypair::generate();
            let kp = identity::Keypair::Ed25519(keypair.clone());
            let peer_id = kp.public().into_peer_id();
            let mut node = Swarm::new(build_memory_transport(kp), node, peer_id);

            for i in 1..10 {
                node.inject_event(
                    RandomPeerId::random(),
                    ConnectionId::new(i),
                    HandlerMessage::InParticle(Particle::default()),
                );
            }

            let node = Arc::new(Mutex::new(node));
            let cfg_parallelism = 4;
            let mut particle_processor = {
                let cloned_node = node.clone();
                particles.for_each_concurrent(cfg_parallelism, move |particle| {
                    let node = cloned_node.clone();
                    async move {
                        let (next_peers, particle) = fce_exec(particle).await;
                        dbg!(&next_peers);
                        for peer in next_peers {
                            let contact = unlock!(node.get_contact(&peer));
                            dbg!(&contact);
                            let contact = match contact {
                                Some(contact) => contact,
                                _ => {
                                    println!("before lock 2");
                                    let contact = unlock!(node.kad_discover(peer).await);
                                    println!("after lock 2");
                                    unlock!(node.connect(contact.clone()).await);
                                    println!("after lock 3");
                                    contact
                                }
                            };

                            dbg!(&contact);

                            unlock!(node.send(contact, particle.clone()));
                        }
                    }
                })
            };

            futures::future::poll_fn::<(), _>(move |cx: &mut Context<'_>| {
                let mut ready = false;

                if let Poll::Ready(mut node) = {
                    println!("poll_fn before node lock");
                    let mut lock = node.lock().boxed();
                    let res = futures::FutureExt::poll_unpin(&mut lock, cx);
                    drop(lock);
                    res
                } {
                    println!("poll_fn node lock READY");
                    ready = dbg!(ExpandedSwarm::poll_next_unpin(&mut node, cx)).is_ready();
                } else {
                    println!("poll_fn node lock PENDING");
                }

                ready = ready
                    || dbg!(futures::FutureExt::poll_unpin(&mut particle_processor, cx)).is_ready();

                if ready {
                    // Return this so task is awaken again immediately
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
        });

        task::block_on(spawned);
    }
}
