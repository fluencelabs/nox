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

use crate::relay::message::RelayMessage;
use crate::relay::protocol::JanusRelay;
use fnv::FnvHashSet;
use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use std::collections::{HashMap, VecDeque};
use std::iter::FromIterator;
use std::marker::PhantomData;
use tokio::prelude::*;

pub(crate) type NetworkState = HashMap<PeerId, FnvHashSet<PeerId>>;

pub struct JanusRelayBehaviour<Substream> {
    // Queue of events to send.
    events: VecDeque<NetworkBehaviourAction<RelayMessage, ()>>,

    /// Connected to this peer nodes.
    connected_nodes: FnvHashSet<PeerId>,

    /// Current network state of all peers with connected nodes.
    network_state: NetworkState,

    /// Pin generic.
    marker: PhantomData<Substream>,
}

impl<Substream> JanusRelayBehaviour<Substream> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            connected_nodes: FnvHashSet::default(),
            network_state: HashMap::new(),
            marker: PhantomData,
        }
    }

    pub fn add_new_peer(&mut self, peer: PeerId, nodes: Vec<PeerId>) {
        self.network_state
            .insert(peer, FnvHashSet::from_iter(nodes));
    }

    pub fn add_new_node(&mut self, peer: &PeerId, node: PeerId) {
        if let Some(v) = self.network_state.get_mut(peer) {
            v.insert(node);
        }
    }

    pub fn remove_node(&mut self, peer: &PeerId, node: PeerId) {
        if let Some(v) = self.network_state.get_mut(peer) {
            v.remove(&node);
        }
    }

    pub fn remove_peer(&mut self, peer: PeerId) {
        self.network_state.remove(&peer);
    }

    pub fn connected_nodes(&self) -> Vec<PeerId> {
        self.connected_nodes.iter().cloned().collect::<Vec<_>>()
    }

    pub fn relay(&mut self, relay_message: RelayMessage) {
        let dst_peer = &relay_message.dst_peer;
        let dst_peer = PeerId::from_bytes(dst_peer.clone()).unwrap();
        if !self.connected_nodes.contains(&dst_peer) {
            for (peer, nodes) in &self.network_state {
                if nodes.contains(&dst_peer) {
                    self.events.push_back(NetworkBehaviourAction::SendEvent {
                        peer_id: peer.to_owned(),
                        event: relay_message.clone(),
                    })
                }
            }
        } else {
            // the destination node is connected to our peer - just send message directly to it
            self.events.push_back(NetworkBehaviourAction::SendEvent {
                peer_id: dst_peer.to_owned(),
                event: relay_message,
            })
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviour for JanusRelayBehaviour<Substream> {
    type ProtocolsHandler = OneShotHandler<Substream, JanusRelay, RelayMessage, InnerMessage>;
    type OutEvent = ();

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
    Rx(RelayMessage),

    /// RelayMessage has been sent
    Tx,
}

impl From<RelayMessage> for InnerMessage {
    #[inline]
    fn from(relay_message: RelayMessage) -> InnerMessage {
        InnerMessage::Rx(relay_message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}
