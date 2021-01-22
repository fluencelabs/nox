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

use crate::connection_pool::{ConnectionPoolT, Contact, LifecycleEvent};

use fluence_libp2p::types::{
    BackPressuredInlet, BackPressuredOutlet, OneshotInlet, OneshotOutlet, Outlet,
};
use fluence_libp2p::{generate_swarm_event_type, remote_multiaddr};
use particle_protocol::{CompletionChannel, HandlerMessage, Particle, ProtocolConfig};
use trust_graph::TrustGraph;

use std::{
    collections::hash_map::Entry,
    collections::{HashMap, HashSet, VecDeque},
    hint::unreachable_unchecked,
    task::{Context, Poll, Waker},
};

use futures::{
    channel::{mpsc, mpsc::SendError, oneshot},
    future,
    future::BoxFuture,
    ready, FutureExt, SinkExt,
};
use libp2p::{
    core::{
        connection::ConnectionId,
        either::EitherOutput::{First, Second},
        ConnectedPoint, Multiaddr,
    },
    identity::{ed25519::Keypair, PublicKey::Ed25519},
    kad::Kademlia,
    swarm::{
        DialPeerCondition, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, NotifyHandler, OneShotHandler, PollParameters,
        ProtocolsHandler,
    },
    PeerId,
};

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
    subscribers: Vec<Outlet<LifecycleEvent>>,

    queue: VecDeque<Particle>,
    contacts: HashMap<PeerId, Peer>,

    events: VecDeque<SwarmEventType>,
    waker: Option<Waker>,
    protocol_config: ProtocolConfig,
}

impl ConnectionPoolBehaviour {
    pub fn connect(&mut self, contact: Contact, outlet: OneshotOutlet<bool>) {
        self.push_event(NetworkBehaviourAction::DialPeer {
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
        };
    }

    pub fn disconnect(&mut self, contact: Contact, _outlet: OneshotOutlet<bool>) {
        todo!(
            "this doesn't make sense with OneShotHandler since connections are short-lived {:?}",
            contact
        )
    }

    pub fn is_connected(&self, peer_id: PeerId, outlet: OneshotOutlet<bool>) {
        outlet.send(self.contacts.contains_key(&peer_id)).ok();
    }

    pub fn get_contact(&self, peer_id: PeerId, outlet: OneshotOutlet<Option<Contact>>) {
        let contact = match self.contacts.get(&peer_id) {
            Some(Peer::Connected(addrs)) => Some(Contact {
                peer_id,
                // TODO: take all addresses
                addr: addrs.iter().next().cloned(),
            }),
            _ => None,
        };
        outlet.send(contact).ok();
    }

    pub fn send(&mut self, to: Contact, particle: Particle, outlet: OneshotOutlet<bool>) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: to.peer_id,
                handler: NotifyHandler::Any,
                event: HandlerMessage::OutParticle(particle, CompletionChannel::Oneshot(outlet)),
            });
        self.wake();
    }

    pub fn count_connections(&mut self, outlet: OneshotOutlet<usize>) {
        outlet.send(self.contacts.len()).ok();
    }

    pub fn add_subscriber(&mut self, outlet: Outlet<LifecycleEvent>) {
        self.subscribers.push(outlet);
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
            subscribers: <_>::default(),
            queue: <_>::default(),
            contacts: <_>::default(),
            events: <_>::default(),
            waker: None,
            protocol_config,
        };

        (this, inlet)
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }

    fn add_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        match self.contacts.entry(peer_id) {
            Entry::Occupied(mut entry) => {
                match entry.get_mut() {
                    Peer::Connected(addrs) => {
                        addrs.insert(addr);
                    }
                    Peer::Dialing(..) => {
                        let mut set = HashSet::new();
                        set.insert(addr);
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
                set.insert(addr);
                e.insert(Peer::Connected(set));
            }
        }
    }

    fn lifecycle_event(&mut self, event: LifecycleEvent) {
        self.subscribers
            .retain(|out| out.unbounded_send(event.clone()).is_ok())
    }

    fn push_event(&mut self, event: SwarmEventType) {
        self.events.push_back(event);
        self.wake();
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
        let addrs = self.contacts.remove(peer_id);
        // TODO: use all addresses, not just the first one
        let addr = addrs
            .into_iter()
            .next()
            .and_then(|p| p.addresses().next().cloned());

        self.lifecycle_event(LifecycleEvent::Connected(Contact {
            peer_id: peer_id.clone(),
            addr,
        }))
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let multiaddr = remote_multiaddr(cp).clone();

        self.add_address(peer_id.clone(), multiaddr.clone());

        self.lifecycle_event(LifecycleEvent::Connected(Contact {
            peer_id: peer_id.clone(),
            addr: multiaddr.into(),
        }))
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
