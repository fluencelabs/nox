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

use fluence_libp2p::generate_swarm_event_type;
use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, OneshotOutlet, Outlet};
use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};

use std::collections::{HashMap, VecDeque};
use std::task::{Context, Poll, Waker};

use futures::channel::mpsc;
use futures::future;
use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput::{First, Second};
use libp2p::core::Multiaddr;
use libp2p::swarm::{
    IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourEventProcess, OneShotHandler,
    PollParameters, ProtocolsHandler,
};
use libp2p::PeerId;

type SwarmEventType = generate_swarm_event_type!(NodeBehaviour);

// #[derive(::libp2p::NetworkBehaviour)]
struct NodeBehaviour {
    pub(super) outlet: BackPressuredOutlet<Particle>,

    pub(super) contacts: HashMap<PeerId, Contact>,

    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ConnectionPool for NodeBehaviour {
    fn connect(&mut self, contact: Contact) -> BoxFuture<'_, bool> {
        todo!()
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

impl NodeBehaviour {
    pub fn new(buffer: usize) -> (Self, BackPressuredInlet<Particle>) {
        let (outlet, inlet) = mpsc::channel(buffer);
        let this = Self {
            outlet,
            contacts: <_>::default(),
            events: VecDeque::<SwarmEventType>::default(),
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

#[cfg(test)]
mod tests {
    use crate::behaviour::NodeBehaviour;
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
            let (mut node, mut particles) = NodeBehaviour::new(100);

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

impl NetworkBehaviour for NodeBehaviour {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.protocol_config.clone().into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, peer_id: &PeerId) {}

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            ProtocolMessage::Particle(particle) => {
                // self.outlet.ingest(particle);
                // self.wake();
            }
            ProtocolMessage::Upgrade => {}
            ProtocolMessage::UpgradeError(err) => log::warn!("UpgradeError: {:?}", err),
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
