/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use crate::bootstrapper::event::BootstrapperEvent;
use janus_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::core::connection::{ConnectedPoint, ConnectionId};
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::{HashSet, VecDeque};
use std::error::Error;

pub type SwarmEventType = generate_swarm_event_type!(Bootstrapper);

pub struct Bootstrapper {
    pub bootstrap_nodes: HashSet<Multiaddr>,
    bootstrap_peers: HashSet<PeerId>,
    events: VecDeque<SwarmEventType>,
}

impl Bootstrapper {
    pub fn new(bootstrap_nodes: Vec<Multiaddr>) -> Self {
        Self {
            bootstrap_nodes: bootstrap_nodes.into_iter().collect(),
            bootstrap_peers: Default::default(),
            events: Default::default(),
        }
    }

    fn push_event(&mut self, event: BootstrapperEvent) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));
    }
}

impl NetworkBehaviour for Bootstrapper {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = BootstrapperEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let maddr = match cp {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        log::debug!("connection established with {} {:?}", peer_id, maddr);

        if self.bootstrap_nodes.contains(maddr) || self.bootstrap_peers.contains(peer_id) {
            self.bootstrap_peers.insert(peer_id.clone());
            self.push_event(BootstrapperEvent::BootstrapConnected {
                peer_id: peer_id.clone(),
                multiaddr: maddr.clone(),
            });
        }
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let maddr = match cp {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        if self.bootstrap_nodes.contains(maddr) || self.bootstrap_peers.contains(peer_id) {
            self.push_event(BootstrapperEvent::BootstrapDisconnected {
                peer_id: peer_id.clone(),
                multiaddr: maddr.clone(),
            });
        }
    }

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, _: void::Void) {}

    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        maddr: &Multiaddr,
        error: &dyn Error,
    ) {
        let is_bootstrap = self.bootstrap_nodes.contains(maddr)
            || peer_id.map_or(false, |id| self.bootstrap_peers.contains(id));

        if is_bootstrap {
            self.push_event(BootstrapperEvent::ReachFailure {
                peer_id: peer_id.cloned(),
                multiaddr: maddr.clone(),
                error: format!("{:?}", error),
            });
        }
    }

    event_polling!(poll, events, SwarmEventType);
}
