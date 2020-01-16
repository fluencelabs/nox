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

use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{
        protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use log::trace;
use std::collections::VecDeque;
use std::marker::PhantomData;
use tokio::prelude::*;
use void::Void;

#[derive(Debug, Clone)]
pub enum SwarmStateEvent {
    Connected(PeerId),
    Disconnected(PeerId),
}

pub struct SwarmStateBehaviour<Substream> {
    // Queue of events to send.
    events: VecDeque<NetworkBehaviourAction<Void, SwarmStateEvent>>,
    /// Pin generic.
    marker: PhantomData<Substream>,
}

impl<Substream> SwarmStateBehaviour<Substream> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            marker: PhantomData,
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviour for SwarmStateBehaviour<Substream> {
    type ProtocolsHandler = DummyProtocolsHandler<Substream>;
    type OutEvent = SwarmStateEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, peer_id: PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/p2p/swarm_state: new peer {} connected",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            SwarmStateEvent::Connected(peer_id),
        ))
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/p2p/swarm_state: peer {} disconnected",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            SwarmStateEvent::Disconnected(peer_id.clone()),
        ))
    }

    fn inject_node_event(&mut self, _source: PeerId, _event: Void) {}

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
