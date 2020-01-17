/*
 * Copyright 2019 Fluence Labs Limited
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

use crate::node_service::relay::events::RelayEvent;
use fnv::FnvHashSet;
use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use log::trace;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::FromIterator;
use std::marker::PhantomData;
use tokio::prelude::*;

pub(crate) type NetworkState = HashMap<PeerId, HashSet<PeerId>>;

/// Behaviour of the Relay layer. Contains the whole network state with connected peers to this
/// node. Produces RelayMessage and save in the internal deque to pass then to the libp2p swarm.
pub struct PeerRelayLayerBehaviour<Substream> {
    // Queue of events to send to the upper level.
    events: VecDeque<NetworkBehaviourAction<RelayEvent, RelayEvent>>,

    /// Connected peers to this node.
    connected_peers: FnvHashSet<PeerId>,

    /// Current network state of all nodes with connected peers.
    network_state: NetworkState,

    marker: PhantomData<Substream>,
}

impl<Substream> PeerRelayLayerBehaviour<Substream> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            connected_peers: FnvHashSet::default(),
            network_state: HashMap::new(),
            marker: PhantomData,
        }
    }

    /// Adds a new node with provided id and a list of connected peers to the network state.
    pub fn add_new_node(&mut self, node_id: PeerId, peer_ids: Vec<PeerId>) {
        self.network_state
            .insert(node_id, HashSet::from_iter(peer_ids));

        self.print_network_state();
    }

    /// Removes node with provided id from the network state.
    pub fn remove_node(&mut self, node_id: &PeerId) {
        self.network_state.remove(node_id);

        self.print_network_state();
    }

    /// Adds a new peer with provided id connected to given node to the network state.
    pub fn add_new_peer(&mut self, node_id: &PeerId, peer_id: PeerId) {
        if let Some(v) = self.network_state.get_mut(node_id) {
            v.insert(peer_id);
        }

        self.print_network_state();
    }

    /// Removes peer with provided id connected to given node from the network state.
    pub fn remove_peer(&mut self, node_id: &PeerId, peer_id: &PeerId) {
        if let Some(v) = self.network_state.get_mut(node_id) {
            v.remove(peer_id);
        }

        self.print_network_state();
    }

    /// Prints the whole network state. Just for debug purposes.
    fn print_network_state(&self) {
        trace!("\nNetwork state:");
        for (node_id, peer_ids) in self.network_state.iter() {
            trace!("node {}, connected peers:", node_id);
            for peer_id in peer_ids.iter() {
                trace!("{}", peer_id);
            }
        }
        trace!("current node, connected peers:");
        for peer_id in self.connected_peers() {
            trace!("{}", peer_id);
        }
    }

    pub fn network_state(&self) -> &NetworkState {
        &self.network_state
    }

    pub fn connected_peers_mut(&mut self) -> &mut FnvHashSet<PeerId> {
        &mut self.connected_peers
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers.iter().cloned().collect::<Vec<_>>()
    }

    /// Relays given message to the given node according to the current network state.
    pub fn relay(&mut self, relay_message: RelayEvent) {
        let dst_peer_id = PeerId::from_bytes(relay_message.dst_id.clone()).unwrap();

        trace!(
            "node_service/relay/behaviour: relaying data to {}",
            dst_peer_id
        );
        if self.connected_peers.contains(&dst_peer_id) {
            // the destination node is connected to our peer - just send message directly to it
            self.events
                .push_back(NetworkBehaviourAction::GenerateEvent(relay_message));
            return;
        }

        for (node, peers) in self.network_state.iter() {
            if peers.contains(&dst_peer_id) {
                self.events.push_back(NetworkBehaviourAction::SendEvent {
                    peer_id: node.to_owned(),
                    event: relay_message,
                });

                return;
            }
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviour for PeerRelayLayerBehaviour<Substream> {
    // use simple one shot handler
    type ProtocolsHandler = OneShotHandler<Substream, RelayEvent, RelayEvent, InnerMessage>;
    type OutEvent = RelayEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _cp: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _cp: ConnectedPoint) {}

    fn inject_node_event(&mut self, _source: PeerId, event: InnerMessage) {
        match event {
            InnerMessage::Rx(relay_message) => self.relay(relay_message),
            InnerMessage::Tx => {}
        }
    }

    fn poll(
        &mut self,
        _: &mut impl PollParameters,
    ) -> Async<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        // events contains RelayMessage events that just need to promoted to the upper level
        if let Some(event) = self.events.pop_front() {
            return Async::Ready(event);
        }

        Async::NotReady
    }
}

/// Transmission between the OneShotHandler message type and the JanusRelay message type.
#[derive(Debug)]
pub enum InnerMessage {
    /// Message has been received from a remote.
    Rx(RelayEvent),

    /// RelayMessage has been sent
    Tx,
}

impl From<RelayEvent> for InnerMessage {
    #[inline]
    fn from(relay_message: RelayEvent) -> InnerMessage {
        InnerMessage::Rx(relay_message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}
