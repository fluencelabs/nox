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

use particle_protocol::{Particle, ProtocolConfig, ProtocolMessage};
use std::collections::VecDeque;
use std::task::{Context, Poll, Waker};

use fluence_libp2p::types::Outlet;
use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput::{First, Second};
use libp2p::core::Multiaddr;
use libp2p::swarm::{
    IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourEventProcess, OneShotHandler,
    PollParameters, ProtocolsHandler,
};
use libp2p::PeerId;

#[derive(::libp2p::NetworkBehaviour)]
struct NodeBehaviour {
    pub(super) outlet: Outlet<Particle>,

    pub(super) events: VecDeque<SwarmEventType>,
    pub(super) waker: Option<Waker>,
    pub(super) protocol_config: ProtocolConfig,
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
        use EitherOutput::{First, Second};

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
