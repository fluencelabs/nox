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

use crate::connection_pool::{ConnectionPoolT, LifecycleEvent};

use fluence_libp2p::types::{
    BackPressuredInlet, BackPressuredOutlet, OneshotInlet, OneshotOutlet, Outlet,
};
use fluence_libp2p::{generate_swarm_event_type, remote_multiaddr};
use particle_protocol::{CompletionChannel, Contact, HandlerMessage, Particle, ProtocolConfig};
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
use std::error::Error;

type SwarmEventType = generate_swarm_event_type!(ConnectionPoolBehaviour);
type Timeout<T> = BoxFuture<'static, T>;

#[derive(Debug)]
enum Peer {
    Connected(HashSet<Multiaddr>),
    /// Storing addresses of connecting peers to return them in `addresses_of_peer`, so libp2p
    /// can ask ConnectionPool about these addresses after `DialPeer` is issued
    Dialing(HashSet<Multiaddr>, Vec<OneshotOutlet<bool>>),
}

impl Peer {
    fn addresses(&self) -> impl Iterator<Item = &Multiaddr> {
        match self {
            Peer::Connected(addrs) => addrs.iter(),
            Peer::Dialing(addrs, _) => addrs.iter(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Peer::Connected(addrs) => addrs.is_empty(),
            Peer::Dialing(addrs, _) => addrs.is_empty(),
        }
    }
}

pub struct ConnectionPoolBehaviour {
    peer_id: PeerId,

    outlet: BackPressuredOutlet<Particle>,
    subscribers: Vec<Outlet<LifecycleEvent>>,

    queue: VecDeque<Particle>,
    contacts: HashMap<PeerId, Peer>,
    dialing: HashMap<Multiaddr, Vec<OneshotOutlet<Option<Contact>>>>,

    events: VecDeque<SwarmEventType>,
    waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
}

impl ConnectionPoolBehaviour {
    /// Dial `address`, and send contact back on success
    /// `None` means something prevented us from connecting - dial reach failure or something else
    pub fn dial(&mut self, address: Multiaddr, out: OneshotOutlet<Option<Contact>>) {
        // TODO: return Contact immediately if that address is already connected
        self.dialing.entry(address.clone()).or_default().push(out);
        self.push_event(NetworkBehaviourAction::DialAddress { address });
    }

    /// Connect to the contact by all of its known addresses and return whether connection succeeded
    /// If contact is already connected, return `true` immediately
    pub fn connect(&mut self, contact: Contact, outlet: OneshotOutlet<bool>) {
        self.push_event(NetworkBehaviourAction::DialPeer {
            peer_id: contact.peer_id,
            condition: DialPeerCondition::Always,
        });

        match self.contacts.entry(contact.peer_id) {
            Entry::Occupied(mut entry) => match entry.get_mut() {
                // TODO: add/replace multiaddr? if yes, do not forget to check connectivity
                Peer::Connected(_) => {
                    outlet.send(true).ok();
                }
                Peer::Dialing(addrs, outlets) => {
                    addrs.extend(contact.addresses);
                    outlets.push(outlet)
                }
            },
            Entry::Vacant(slot) => {
                slot.insert(Peer::Dialing(
                    contact.addresses.into_iter().collect(),
                    vec![outlet],
                ));
            }
        };
    }

    // TODO: implement
    pub fn disconnect(&mut self, contact: Contact, _outlet: OneshotOutlet<bool>) {
        todo!(
            "this doesn't make sense with OneShotHandler since connections are short-lived {:?}",
            contact
        )
    }

    /// Returns whether given peer is connected or not
    pub fn is_connected(&self, peer_id: PeerId, outlet: OneshotOutlet<bool>) {
        outlet.send(self.contacts.contains_key(&peer_id)).ok();
    }

    /// Returns contact for a given peer if it is known
    pub fn get_contact(&self, peer_id: PeerId, outlet: OneshotOutlet<Option<Contact>>) {
        let contact = self.get_contact_impl(peer_id);
        outlet.send(contact).ok();
    }

    /// Sends a particle to a connected contact. Returns whether sending succeeded or not
    /// Result is sent to channel inside `upgrade_outbound` in ProtocolHandler
    pub fn send(&mut self, to: Contact, particle: Particle, outlet: OneshotOutlet<bool>) {
        if to.peer_id == self.peer_id {
            // If particle is sent to the current node, process it locally
            self.queue.push_back(particle);
            outlet.send(true).ok();
            self.wake();
        } else {
            // Send particle to remote peer
            self.push_event(NetworkBehaviourAction::NotifyHandler {
                peer_id: to.peer_id,
                handler: NotifyHandler::Any,
                event: HandlerMessage::OutParticle(particle, CompletionChannel::Oneshot(outlet)),
            });
        }
    }

    /// Returns number of connected contacts
    pub fn count_connections(&mut self, outlet: OneshotOutlet<usize>) {
        outlet.send(self.contacts.len()).ok();
    }

    /// Subscribes given channel for all `LifecycleEvent`s
    pub fn add_subscriber(&mut self, outlet: Outlet<LifecycleEvent>) {
        self.subscribers.push(outlet);
    }
}

impl ConnectionPoolBehaviour {
    pub fn new(
        buffer: usize,
        protocol_config: ProtocolConfig,
        peer_id: PeerId,
    ) -> (Self, BackPressuredInlet<Particle>) {
        let (outlet, inlet) = mpsc::channel(buffer);

        let this = Self {
            peer_id,
            outlet,
            subscribers: <_>::default(),
            queue: <_>::default(),
            contacts: <_>::default(),
            dialing: <_>::default(),
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
        // notify these waiting for a peer to be connected
        match self.contacts.entry(peer_id) {
            Entry::Occupied(mut entry) => {
                match entry.get_mut() {
                    Peer::Connected(addrs) => {
                        addrs.insert(addr.clone());
                    }
                    Peer::Dialing(..) => {
                        let mut set = HashSet::new();
                        set.insert(addr.clone());
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
                set.insert(addr.clone());
                e.insert(Peer::Connected(set));
            }
        }

        // notify these waiting for an address to be dialed
        if let Some(outs) = self.dialing.remove(&addr) {
            let contact = self.get_contact_impl(peer_id);
            debug_assert!(contact.is_some());
            for out in outs {
                out.send(contact.clone()).ok();
            }
        }
    }

    fn lifecycle_event(&mut self, event: LifecycleEvent) {
        self.subscribers.retain(|out| {
            let ok = out.unbounded_send(event.clone());
            ok.is_ok()
        })
    }

    fn push_event(&mut self, event: SwarmEventType) {
        self.events.push_back(event);
        self.wake();
    }

    fn remove_contact(&mut self, peer_id: &PeerId, reason: &str) {
        if let Some(contact) = self.contacts.remove(peer_id) {
            log::debug!("Contact {} was removed: {}", peer_id, reason);
            let addresses = match contact {
                Peer::Connected(addrs) => addrs,
                Peer::Dialing(addrs, outs) => {
                    // if dial was in progress, notify waiters
                    for out in outs {
                        out.send(false).ok();
                    }

                    addrs
                }
            };

            self.lifecycle_event(LifecycleEvent::Disconnected(Contact::new(
                peer_id.clone(),
                addresses.into_iter().collect(),
            )))
        }
    }

    fn get_contact_impl(&self, peer_id: PeerId) -> Option<Contact> {
        match self.contacts.get(&peer_id) {
            Some(Peer::Connected(addrs)) => {
                Some(Contact::new(peer_id, addrs.into_iter().cloned().collect()))
            }
            _ => None,
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

    fn inject_connected(&mut self, peer_id: &PeerId) {
        // NOTE: `addresses_of_peer` at this point must be filled
        // with addresses through inject_connection_established
        let contact = Contact::new(peer_id.clone(), self.addresses_of_peer(peer_id));
        debug_assert!(!contact.addresses.is_empty());
        // Signal a new peer connected
        self.lifecycle_event(LifecycleEvent::Connected(contact));
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.remove_contact(peer_id, "disconnected");
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let multiaddr = remote_multiaddr(cp).clone();

        self.add_address(*peer_id, multiaddr.clone());

        self.lifecycle_event(LifecycleEvent::Connected(Contact::new(
            *peer_id,
            vec![multiaddr],
        )))
    }

    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        addr: &Multiaddr,
        error: &dyn Error,
    ) {
        let peer = peer_id
            .map(|id| format!(" peer id {}", id))
            .unwrap_or_default();
        log::warn!("failed to connect to {}{}: {}", addr, peer, error);

        if let Some(peer_id) = peer_id {
            let empty = self.contacts.get_mut(peer_id).map_or(false, |contact| {
                // remove failed address
                match contact {
                    Peer::Connected(addrs) => addrs.remove(addr),
                    Peer::Dialing(addrs, _) => addrs.remove(addr),
                };
                contact.is_empty()
            });

            // if contact is empty (there are no addresses), remove it
            if empty {
                self.remove_contact(
                    peer_id,
                    format!("address {} reach failure {}", addr, error).as_str(),
                );
            }
        }

        // Notify those who waits for address dial
        if let Some(outs) = self.dialing.remove(addr) {
            for out in outs {
                out.send(None).ok();
            }
        }
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        // remove failed contact
        self.remove_contact(peer_id, "dial failure, no more addresses to try")
    }

    fn inject_event(
        &mut self,
        from: PeerId,
        _: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            HandlerMessage::InParticle(particle) => {
                log::trace!(target: "network", "received particle {} from {}", particle.id, from);
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
                    if self.outlet.is_closed() {
                        log::error!("Particle outlet closed");
                    }
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
