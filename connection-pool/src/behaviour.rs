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
use futures::FutureExt;
use futures::{future, SinkExt};
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput::{First, Second};
use libp2p::core::{ConnectedPoint, Multiaddr};
use libp2p::swarm::{
    DialPeerCondition, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
    NetworkBehaviourEventProcess, OneShotHandler, PollParameters, ProtocolsHandler,
};
use libp2p::PeerId;

type SwarmEventType = generate_swarm_event_type!(ConnectionPoolBehaviour);

enum Peer {
    Connected(Multiaddr),
    Dialing(Multiaddr, OneshotOutlet<bool>),
}

// #[derive(::libp2p::NetworkBehaviour)]
struct ConnectionPoolBehaviour {
    pub(super) outlet: BackPressuredOutlet<Particle>,
    pub(super) queue: VecDeque<Particle>,

    pub(super) contacts: HashMap<PeerId, Multiaddr>,

    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ConnectionPool for ConnectionPoolBehaviour {
    fn connect(&mut self, contact: Contact) -> BoxFuture<'_, bool> {
        let (outlet, inlet) = futures::channel::oneshot::channel();
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id: contact.peer_id,
            condition: DialPeerCondition::Always,
        });

        inlet.await
    }

    fn disconnect(&mut self, contact: Contact) -> BoxFuture<'_, bool> {
        todo!()
    }

    fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.contacts.contains_key(peer_id)
    }

    fn get_contact(&self, peer_id: &PeerId) -> Option<Contact> {
        todo!()
    }

    fn send(&mut self, to: Contact, particle: Particle) -> BoxFuture<'_, bool> {
        todo!()
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
}

impl NetworkBehaviour for ConnectionPoolBehaviour {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.protocol_config.clone().into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.contacts.get(peer_id).into_iter().cloned().collect()
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
        self.contacts.insert(peer_id.clone(), multiaddr.clone());
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
            match self.outlet.poll_ready(cx) {
                Poll::Ready(Ok(_)) => {
                    if let Some(particle) = self.queue.pop_front() {
                        self.outlet.start_send(particle);
                    } else {
                        break;
                    }
                }
                Poll::Ready(Err(err)) => {
                    log::warn!("ConnectionPool particle inlet has been dropped: {}", err);
                    break;
                }
                Poll::Pending => {
                    if self.queue.len() > 100 {
                        log::warn!("Particle queue seems to have stalled");
                    }
                    break;
                }
            }
        }

        while let Some(Ok(_)) = self.outlet.poll_ready(cx) {
            if let Some(particle) = self.queue.pop_front() {
                self.outlet.start_send(particle)
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
    use futures::future::BoxFuture;
    use futures::FutureExt;
    use futures::StreamExt;
    use libp2p::PeerId;
    use particle_protocol::Particle;

    fn fce_exec(particle: Particle) -> BoxFuture<'static, (Vec<PeerId>, Particle)> {
        futures::future::ready((vec![particle.init_peer_id.clone()], particle)).boxed()
    }

    #[test]
    fn run() {
        let spawned = task::spawn(async move {
            let (mut node, mut particles) = ConnectionPoolBehaviour::new(100);

            loop {
                if let Some(particle) = particles.next().await {
                    let (next_peers, particle) = fce_exec(particle).await;
                    for peer in next_peers {
                        let contact = match node.get_contact(&peer) {
                            Some(contact) => contact,
                            _ => {
                                let contact = node.kad_discover(peer).await;
                                node.connect(contact.clone()).await;
                                contact
                            }
                        };

                        node.send(contact, particle.clone()).await;
                    }
                }
            }
        });

        task::block_on(spawned);
    }
}
