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

use crate::node_service::connect_protocol::behaviour::NodeConnectProtocolBehaviour;
use crate::node_service::events::OutNodeServiceEvent;
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::{NetworkBehaviour, PeerId};
use std::collections::VecDeque;
use tokio::prelude::*;

#[derive(NetworkBehaviour)]
pub struct NodeServiceBehaviour<Substream: AsyncRead + AsyncWrite> {
    ping: Ping<Substream>,
    identity: Identify<Substream>,
    pub node_connect_protocol: NodeConnectProtocolBehaviour<Substream>,

    #[behaviour(ignore)]
    pub nodes_events: VecDeque<OutNodeServiceEvent>,
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<OutNodeServiceEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, event: OutNodeServiceEvent) {
        self.nodes_events.push_back(event);
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: PingEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for NodeServiceBehaviour<Substream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl<Substream: AsyncRead + AsyncWrite> NodeServiceBehaviour<Substream> {
    pub fn new(_local_peer_id: PeerId, local_public_key: PublicKey) -> Self {
        let ping = Ping::new(PingConfig::new());
        let identity = Identify::new(
            "/janus/node_connect/1.0.0".into(),
            "janus".into(),
            local_public_key,
        );
        let node_connect_protocol = NodeConnectProtocolBehaviour::new();

        Self {
            ping,
            identity,
            node_connect_protocol,
            nodes_events: VecDeque::new(),
        }
    }

    pub fn exit(&mut self) {
        unimplemented!();
    }
}
