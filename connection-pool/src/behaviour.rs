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

use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    error::Error,
    task::{Context, Poll, Waker},
};

use futures::channel::mpsc;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::DialError;
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint, Multiaddr},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
        ProtocolsHandler,
    },
    PeerId,
};

use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, OneshotOutlet, Outlet};
use fluence_libp2p::{generate_swarm_event_type, remote_multiaddr};
use particle_protocol::{CompletionChannel, Contact, HandlerMessage, Particle, ProtocolConfig};

use crate::connection_pool::LifecycleEvent;

// type SwarmEventType = generate_swarm_event_type!(ConnectionPoolBehaviour);

// TODO: replace with generate_swarm_event_type
type SwarmEventType = libp2p::swarm::NetworkBehaviourAction<
    (),
    OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage>,
>;

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

        let handler = self.new_handler();
        self.push_event(NetworkBehaviourAction::Dial {
            opts: DialOpts::unknown_peer_id().address(address).build(),
            handler,
        });
    }

    /// Connect to the contact by all of its known addresses and return whether connection succeeded
    /// If contact is already connected, return `true` immediately
    pub fn connect(&mut self, contact: Contact, outlet: OneshotOutlet<bool>) {
        let handler = self.new_handler();
        self.push_event(NetworkBehaviourAction::Dial {
            opts: DialOpts::peer_id(contact.peer_id)
                .addresses(contact.addresses.clone())
                .build(),
            handler,
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
        } else if self.contacts.contains_key(&to.peer_id) {
            // Send particle to remote peer
            self.push_event(NetworkBehaviourAction::NotifyHandler {
                peer_id: to.peer_id,
                handler: NotifyHandler::Any,
                event: HandlerMessage::OutParticle(particle, CompletionChannel::Oneshot(outlet)),
            });
        } else {
            log::warn!(
                "Won't send particle {} to contact {}: not connected",
                particle.id,
                to.peer_id
            );
            outlet.send(false).ok();
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
                *peer_id,
                addresses.into_iter().collect(),
            )))
        }
    }

    fn get_contact_impl(&self, peer_id: PeerId) -> Option<Contact> {
        match self.contacts.get(&peer_id) {
            Some(Peer::Connected(addrs)) => {
                Some(Contact::new(peer_id, addrs.iter().cloned().collect()))
            }
            _ => None,
        }
    }

    fn fail_address(&mut self, peer_id: &PeerId, addr: &Multiaddr) {
        log::warn!("failed to connect to {} {}", addr, peer_id);

        let contact = self.contacts.get_mut(peer_id);
        // remove failed address
        match contact {
            Some(Peer::Connected(addrs)) | Some(Peer::Dialing(addrs, _)) => {
                addrs.remove(addr);
            }
            None => {}
        };

        // Notify those who waits for address dial
        if let Some(outs) = self.dialing.remove(addr) {
            for out in outs {
                out.send(None).ok();
            }
        }
    }
}

impl NetworkBehaviour for ConnectionPoolBehaviour {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        self.protocol_config.clone().into()
    }

    // TODO: seems like there's no need in that method anymore IFF it is used only for dialing
    //       see https://github.com/libp2p/rust-libp2p/blob/master/swarm/CHANGELOG.md#0320-2021-11-16
    //       ACTION: remove this method. ALSO: remove `self.contacts`?
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
        let contact = Contact::new(*peer_id, self.addresses_of_peer(peer_id));
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
        _connection_id: &ConnectionId,
        cp: &ConnectedPoint,
        failed_addresses: Option<&Vec<Multiaddr>>,
    ) {
        // mark failed addresses as such
        if let Some(failed_addresses) = failed_addresses {
            for addr in failed_addresses {
                self.fail_address(peer_id, addr)
            }
        }

        let multiaddr = remote_multiaddr(cp).clone();

        self.add_address(*peer_id, multiaddr.clone());

        self.lifecycle_event(LifecycleEvent::Connected(Contact::new(
            *peer_id,
            vec![multiaddr],
        )))
    }

    fn inject_dial_failure(
        &mut self,
        peer_id: Option<PeerId>,
        _handler: Self::ProtocolsHandler,
        error: &DialError,
    ) {
        // remove failed contact
        if let Some(peer_id) = peer_id {
            self.remove_contact(&peer_id, format!("dial failure: {}", error).as_str())
        } else {
            log::warn!("Unknown peer dial failure: {}", error)
        }
    }

    fn inject_event(
        &mut self,
        from: PeerId,
        _: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            HandlerMessage::InParticle(particle) => {
                log::trace!(target: "network", "received particle {} from {}; queue {}", particle.id, from, self.queue.len());
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

        loop {
            // Check backpressure on the outlet
            match self.outlet.poll_ready(cx) {
                Poll::Ready(Ok(_)) => {
                    // channel is ready to consume more particles, so send them
                    if let Some(particle) = self.queue.pop_front() {
                        let particle_id = particle.id.clone();
                        if let Err(err) = self.outlet.start_send(particle) {
                            log::error!("Failed to send particle to outlet: {}", err)
                        } else {
                            log::trace!(target: "network", "Sent particle {} to execution", particle_id);
                        }
                    } else {
                        break;
                    }
                }
                Poll::Pending => {
                    // if channel is full, then keep particles in the queue
                    let len = self.queue.len();
                    if len > 30 {
                        log::warn!("Particle queue seems to have stalled; queue {}", len);
                    } else {
                        log::trace!(target: "network", "Connection pool outlet is pending; queue {}", len);
                    }
                    if self.outlet.is_closed() {
                        log::error!("Particle outlet closed");
                    }
                    break;
                }
                Poll::Ready(Err(err)) => {
                    log::warn!("ConnectionPool particle inlet has been dropped: {}", err);
                    break;
                }
            }
        }

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
