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

use crate::actors::plumber::Plumber;
use fluence_libp2p::generate_swarm_event_type;
use futures::task::{Context, Poll};
use libp2p::{
    core::connection::ConnectionId,
    swarm::{
        IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, OneShotHandler,
        PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use parity_multiaddr::Multiaddr;
use particle_protocol::{ProtocolConfig, ProtocolMessage};

pub(crate) type SwarmEventType = generate_swarm_event_type!(Plumber);

impl NetworkBehaviour for Plumber {
    type ProtocolsHandler = OneShotHandler<ProtocolConfig, ProtocolMessage, ProtocolMessage>;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        let config = ProtocolConfig::new();
        config.into()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        unimplemented!()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        unimplemented!()
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        unimplemented!()
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        unimplemented!()
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        unimplemented!()
    }
}
