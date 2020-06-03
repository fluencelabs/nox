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

use crate::bootstrapper::event::BootstrapperEvent;
use fluence_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::core::connection::{ConnectedPoint, ConnectionId};
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::mem;
use std::time::{Duration, Instant};

pub type SwarmEventType = generate_swarm_event_type!(Bootstrapper);
// TODO: make it exponential
static RECONNECT_DELAY: Option<Duration> = Some(Duration::from_millis(1500));

pub struct Bootstrapper {
    pub bootstrap_nodes: HashSet<Multiaddr>,
    bootstrap_peers: HashSet<PeerId>,
    delayed_events: Vec<(Option<Instant>, SwarmEventType)>,
    events: VecDeque<SwarmEventType>,
}

impl Bootstrapper {
    pub fn new(bootstrap_nodes: Vec<Multiaddr>) -> Self {
        Self {
            bootstrap_nodes: bootstrap_nodes.into_iter().collect(),
            bootstrap_peers: Default::default(),
            delayed_events: Default::default(),
            events: Default::default(),
        }
    }

    fn push_event(&mut self, event: BootstrapperEvent, delay: Option<Duration>) {
        let event = NetworkBehaviourAction::GenerateEvent(event);
        let deadline = delay.map(|d| Instant::now() + d);
        self.delayed_events.push((deadline, event));
    }

    fn complete_delayed(&mut self) {
        let now = Instant::now();

        let delayed = mem::replace(&mut self.delayed_events, vec![]);

        let (ready, not_ready) = delayed.into_iter().partition(|(deadline, _)| {
            let ready = deadline.map(|d| d >= now).unwrap_or(true);
            ready
        });

        self.delayed_events = not_ready;
        self.events = ready.into_iter().map(|(_, e)| e).collect();
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
            self.push_event(
                BootstrapperEvent::BootstrapConnected {
                    peer_id: peer_id.clone(),
                    multiaddr: maddr.clone(),
                },
                None,
            );
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
            self.push_event(
                BootstrapperEvent::ReconnectToBootstrap {
                    peer_id: Some(peer_id.clone()),
                    multiaddr: maddr.clone(),
                    error: None,
                },
                RECONNECT_DELAY,
            );
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
            self.push_event(
                BootstrapperEvent::ReconnectToBootstrap {
                    peer_id: peer_id.cloned(),
                    multiaddr: maddr.clone(),
                    error: Some(format!("{:?}", error)),
                },
                RECONNECT_DELAY,
            );
        }
    }

    event_polling!(poll, events, SwarmEventType, complete_delayed);
}
