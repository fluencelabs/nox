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

use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, OneshotOutlet, Outlet};
use fluence_libp2p::{generate_swarm_event_type, remote_multiaddr};
use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};

use std::collections::{HashMap, VecDeque};
use std::task::{Context, Poll, Waker};

use futures::channel::mpsc;
use futures::channel::mpsc::SendError;
use futures::future::BoxFuture;
use futures::{future, SinkExt};
use futures::{FutureExt, TryFutureExt};
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput::{First, Second};
use libp2p::core::{ConnectedPoint, Multiaddr};
use libp2p::swarm::{
    DialPeerCondition, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
    NetworkBehaviourEventProcess, NotifyHandler, OneShotHandler, PollParameters, ProtocolsHandler,
};
use libp2p::PeerId;
use std::collections::hash_map::Entry;

type SwarmEventType = generate_swarm_event_type!(ConnectionPoolBehaviour);

#[derive(Debug)]
enum Peer {
    Connected(Multiaddr),
    Dialing(Option<Multiaddr>, Vec<OneshotOutlet<bool>>),
}

impl Peer {
    fn multiaddr(&self) -> Option<&Multiaddr> {
        match self {
            Peer::Connected(maddr) => Some(maddr),
            Peer::Dialing(maddr, _) => maddr.as_ref(),
        }
    }
}

// #[derive(::libp2p::NetworkBehaviour)]
struct ConnectionPoolBehaviour {
    pub(super) outlet: BackPressuredOutlet<Particle>,
    pub(super) queue: VecDeque<Particle>,

    pub(super) contacts: HashMap<PeerId, Peer>,

    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ConnectionPool for ConnectionPoolBehaviour {
    fn connect(&mut self, contact: Contact) -> BoxFuture<'_, bool> {
        let (outlet, inlet) = futures::channel::oneshot::channel();
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id: contact.peer_id.clone(),
            condition: DialPeerCondition::Always,
        });

        match self.contacts.entry(contact.peer_id.clone()) {
            Entry::Occupied(mut entry) => match entry.get_mut() {
                // TODO: add/replace multiaddr? if yes, do not forget to check connectivity
                Peer::Connected(_) => return future::ready(true).boxed(),
                Peer::Dialing(_, outlets) => outlets.push(outlet),
            },
            Entry::Vacant(slot) => {
                slot.insert(Peer::Dialing(contact.addr, vec![outlet]));
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

    fn disconnect(&mut self, contact: Contact) -> BoxFuture<'_, bool> {
        todo!("haha, libp2p won't allow me doing that! {:?}", contact)
    }

    fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.contacts.contains_key(peer_id)
    }

    fn get_contact(&self, peer_id: &PeerId) -> Option<Contact> {
        match self.contacts.get(peer_id) {
            Some(Peer::Connected(maddr)) => Some(Contact {
                peer_id: peer_id.clone(),
                addr: maddr.clone().into(),
            }),
            _ => None,
        }
    }

    fn send(&mut self, to: Contact, particle: Particle) -> BoxFuture<'_, bool> {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: to.peer_id,
                handler: NotifyHandler::Any,
                event: ProtocolMessage::Particle(particle),
            });

        // TODO: how to check if a message was delivered? I mean... implementing custom ProtocolHandler, right?
        //       gotta try RequestResponse handler.
        future::ready(true).boxed()
    }
}

impl ConnectionPoolBehaviour {
    pub fn new(buffer: usize) -> (Self, BackPressuredInlet<Particle>) {
        let (outlet, inlet) = mpsc::channel(buffer);
        let this = Self {
            outlet,
            queue: <_>::default(),
            contacts: <_>::default(),
            events: <_>::default(),
            waker: None,
            protocol_config: <_>::default(),
        };

        (this, inlet)
    }

    pub fn kad_discover(&self, peer_id: PeerId) -> BoxFuture<'_, Contact> {
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
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.protocol_config.clone().into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.contacts
            .get(peer_id)
            .into_iter()
            .flat_map(|p| p.multiaddr().cloned())
            .collect()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.contacts.remove(peer_id);
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let multiaddr = remote_multiaddr(cp);
        self.contacts
            .insert(peer_id.clone(), Peer::Connected(multiaddr.clone()));
    }

    fn inject_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            ProtocolMessage::Particle(particle) => {
                self.queue.push_back(particle);
                self.wake();
            }
            ProtocolMessage::Upgrade => {}
            ProtocolMessage::UpgradeError(err) => log::warn!("UpgradeError: {:?}", err),
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

    use async_std::task;
    use fluence_libp2p::{build_memory_transport, RandomPeerId};
    use futures::future::BoxFuture;
    use futures::select;
    use futures::FutureExt;
    use futures::StreamExt;
    use libp2p::core::connection::ConnectionId;
    use libp2p::identity::ed25519::Keypair;
    use libp2p::swarm::NetworkBehaviour;
    use libp2p::{identity, PeerId, Swarm};
    use particle_protocol::{Particle, ProtocolMessage};

    fn fce_exec(particle: Particle) -> BoxFuture<'static, (Vec<PeerId>, Particle)> {
        futures::future::ready((vec![particle.init_peer_id.clone()], particle)).boxed()
    }

    #[test]
    fn run() {
        let spawned = task::spawn(async move {
            let (node, particles) = ConnectionPoolBehaviour::new(100);
            let keypair = Keypair::generate();
            let kp = identity::Keypair::Ed25519(keypair.clone());
            let peer_id = kp.public().into_peer_id();
            let mut node = Swarm::new(build_memory_transport(kp), node, peer_id);
            node.inject_event(
                RandomPeerId::random(),
                ConnectionId::new(0),
                ProtocolMessage::Particle(Particle::default()),
            );

            node.inject_event(
                RandomPeerId::random(),
                ConnectionId::new(0),
                ProtocolMessage::Particle(Particle::default()),
            );

            let mut particles = particles.fuse();

            loop {
                select! {
                    particle = particles.next() => {
                        dbg!(&particle);
                        if let Some(particle) = particle {
                            let (next_peers, particle) = fce_exec(particle).await;
                            dbg!(&next_peers);
                            for peer in next_peers {
                                let contact = match node.get_contact(&peer) {
                                    Some(contact) => contact,
                                    _ => {
                                        let contact = node.kad_discover(peer).await;
                                        node.connect(contact.clone()).await;
                                        contact
                                    }
                                };

                                dbg!(&contact);

                                node.send(contact, particle.clone()).await;
                            }
                        }
                    },
                    _ = node.select_next_some() => {

                    }
                };
            }
        });

        task::block_on(spawned);
    }
}
