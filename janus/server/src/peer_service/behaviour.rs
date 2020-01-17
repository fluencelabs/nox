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

use crate::peer_service::connect_protocol::behaviour::PeerConnectProtocolBehaviour;
use crate::peer_service::notifications::OutPeerNotification;
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::{NetworkBehaviour, PeerId};
use std::collections::VecDeque;
use tokio::prelude::*;

#[derive(NetworkBehaviour)]
pub struct PeerServiceBehaviour<Substream: AsyncRead + AsyncWrite> {
    ping: Ping<Substream>,
    identity: Identify<Substream>,
    node_connect_protocol: PeerConnectProtocolBehaviour<Substream>,

    #[behaviour(ignore)]
    nodes_events: VecDeque<OutPeerNotification>,
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<OutPeerNotification>
    for PeerServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: OutPeerNotification) {
        self.nodes_events.push_back(event);
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for PeerServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for PeerServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> PeerServiceBehaviour<Substream> {
    pub fn new(_local_peer_id: &PeerId, local_public_key: PublicKey) -> Self {
        let ping = Ping::new(PingConfig::new());
        let identity = Identify::new("1.0.0".into(), "1.0.0".into(), local_public_key);
        let node_connect_protocol = PeerConnectProtocolBehaviour::new();

        Self {
            ping,
            identity,
            node_connect_protocol,
            nodes_events: VecDeque::new(),
        }
    }

    pub fn pop_out_node_event(&mut self) -> Option<OutPeerNotification> {
        self.nodes_events.pop_front()
    }

    pub fn relay_message(&mut self, src: PeerId, dst: PeerId, message: Vec<u8>) {
        self.node_connect_protocol.relay_message(src, dst, message);
    }

    pub fn send_network_state(&mut self, dst: PeerId, state: Vec<PeerId>) {
        self.node_connect_protocol.send_network_state(dst, state);
    }

    pub fn exit(&mut self) {
        unimplemented!();
    }
}
