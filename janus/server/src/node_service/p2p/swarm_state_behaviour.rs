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

use crate::event_polling;
use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{
        protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
        ProtocolsHandler,
    },
    PeerId,
};
use log::trace;
use parity_multiaddr::Protocol;
use std::collections::{HashMap, HashSet, VecDeque};
use void::Void;

#[derive(Debug, Clone)]
pub enum SwarmStateEvent {
    Connected { id: PeerId },
    Disconnected { id: PeerId },
}

/// The main purpose of this behaviour is to emit events about connecting/disconnecting of nodes.
/// It seems that for rust-libp2p 0.14 it is the easiest way to find out the PeerId while node
/// connecting and disconnecting.
pub struct SwarmStateBehaviour {
    // Queue of events to send to the upper level.
    events: VecDeque<NetworkBehaviourAction<Void, SwarmStateEvent>>,
    addrs: HashMap<PeerId, HashSet<Multiaddr>>,
    node_service_port: u16,
}

impl SwarmStateBehaviour {
    pub fn add_node_addresses(&mut self, node_id: PeerId, addrs: HashSet<Multiaddr>) {
        self.addrs.insert(node_id, addrs);
    }
}

impl SwarmStateBehaviour {
    pub fn new(node_service_port: u16) -> Self {
        Self {
            events: VecDeque::new(),
            addrs: HashMap::new(),
            node_service_port,
        }
    }
}

impl NetworkBehaviour for SwarmStateBehaviour {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = SwarmStateEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.addrs
            .get(peer_id)
            .get_or_insert(&HashSet::new())
            .iter()
            // excess copy here - waiting for https://github.com/libp2p/rust-libp2p/issues/1417
            .cloned()
            .map(|addr| addr)
            .collect::<Vec<Multiaddr>>()
    }

    fn inject_connected(&mut self, node_id: PeerId, cp: ConnectedPoint) {
        match cp {
            ConnectedPoint::Listener {
                local_addr: _local_addr,
                mut send_back_addr,
            } => {
                // replace incoming listener port to the node service port
                // TODO: check the popped protocol to equal tcp
                send_back_addr.pop();
                send_back_addr.push(Protocol::Tcp(self.node_service_port));

                trace!(
                    "node_service/p2p/swarm_state: new node {} connected with multiaddr {:?}",
                    node_id,
                    send_back_addr
                );

                self.addrs
                    .entry(node_id.clone())
                    .or_insert_with(HashSet::new)
                    .insert(send_back_addr);
            }
            ConnectedPoint::Dialer { .. } => {}
        }

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            SwarmStateEvent::Connected { id: node_id },
        ));
    }

    fn inject_disconnected(&mut self, node_id: &PeerId, _cp: ConnectedPoint) {
        trace!(
            "node_service/p2p/swarm_state: node {} disconnected",
            node_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            SwarmStateEvent::Disconnected {
                id: node_id.to_owned(),
            },
        ))
    }

    fn inject_node_event(&mut self, _source: PeerId, _event: Void) {}

    // produces SwarmStateEvent
    event_polling!(
        poll,
        events,
        NetworkBehaviourAction<<Self::ProtocolsHandler as ProtocolsHandler>::InEvent, Self::OutEvent>
    );
}
