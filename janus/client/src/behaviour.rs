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

use crate::connect_protocol::behaviour::ClientConnectProtocolBehaviour;
use crate::connect_protocol::events::InEvent;
use futures::{AsyncRead, AsyncWrite};
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::{NetworkBehaviour, PeerId};
use std::collections::VecDeque;

#[derive(NetworkBehaviour)]
pub struct ClientServiceBehaviour<Substream: AsyncRead + AsyncWrite> {
    ping: Ping<Substream>,
    identity: Identify<Substream>,
    node_connect_protocol: ClientConnectProtocolBehaviour<Substream>,

    #[behaviour(ignore)]
    nodes_events: VecDeque<InEvent>,
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<InEvent>
    for ClientServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: InEvent) {
        self.nodes_events.push_back(event);
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for ClientServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for ClientServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> ClientServiceBehaviour<Substream> {
    pub fn new(_local_peer_id: &PeerId, local_public_key: PublicKey) -> Self {
        let ping = Ping::new(
            PingConfig::new()
                .with_max_failures(unsafe { core::num::NonZeroU32::new_unchecked(10) }),
        );
        let identity = Identify::new("1.0.0".into(), "1.0.0".into(), local_public_key);
        let node_connect_protocol = ClientConnectProtocolBehaviour::new();

        Self {
            ping,
            identity,
            node_connect_protocol,
            nodes_events: VecDeque::new(),
        }
    }

    pub fn pop_out_node_event(&mut self) -> Option<InEvent> {
        self.nodes_events.pop_front()
    }

    pub fn send_message(&mut self, relay: PeerId, dst: PeerId, message: Vec<u8>) {
        self.node_connect_protocol.send_message(relay, dst, message);
    }

    pub fn get_network_state(&mut self, relay: PeerId) {
        self.node_connect_protocol.get_network_state(relay);
    }

    pub fn exit(&mut self) {
        unimplemented!();
    }
}
