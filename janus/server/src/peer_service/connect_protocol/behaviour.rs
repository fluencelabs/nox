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
use crate::generate_swarm_event_type;
use crate::peer_service::connect_protocol::messages::{ToNodeNetworkMsg, ToPeerNetworkMsg};
use crate::peer_service::messages::ToNodeMsg;
use libp2p::{
    core::connection::ConnectionId,
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler},
    PeerId,
};
use log::trace;
use multihash::Multihash;
use std::collections::VecDeque;

type SwarmEventType = generate_swarm_event_type!(PeerConnectBehaviour);

#[derive(Default)]
pub struct PeerConnectBehaviour {
    /// Queue of received network messages from connected peers
    /// that need to be handled during polling.
    events: VecDeque<SwarmEventType>,
}

impl PeerConnectBehaviour {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn deliver_data(&mut self, src: PeerId, dst: PeerId, data: Vec<u8>) {
        self.send_event(dst, ToPeerNetworkMsg::Deliver { src_id: src, data });
    }

    /// Deliver FindProviders result to connected peer
    /// `client_id` peer id of the client issued a request
    /// `peer_id` key to find providers for
    /// `providers` list of addresses and peer ids of providers
    pub fn deliver_providers(
        &mut self,
        client_id: PeerId,
        key: Multihash,
        providers: Vec<(Multiaddr, PeerId)>,
    ) {
        self.send_event(
            client_id.clone(),
            ToPeerNetworkMsg::Providers {
                client_id,
                key,
                providers,
            },
        )
    }

    fn send_event(&mut self, peer_id: PeerId, event: ToPeerNetworkMsg) {
        trace!(
            "peer_service/connect_protocol/behaviour: delivering event {:?} to {:?}",
            event,
            peer_id
        );

        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                event,
                handler: NotifyHandler::Any,
            })
    }

    fn enqueue_event(&mut self, event: ToNodeMsg) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event))
    }
}

impl NetworkBehaviour for PeerConnectBehaviour {
    type ProtocolsHandler = OneShotHandler<ToNodeNetworkMsg, ToPeerNetworkMsg, InnerMessage>;
    type OutEvent = ToNodeMsg;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, peer_id: PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/connect_protocol/inject_connected: new peer {} joined",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            ToNodeMsg::PeerConnected { peer_id },
        ));
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/connect_protocol/inject_disconnected: peer {} disconnected",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            ToNodeMsg::PeerDisconnected {
                peer_id: peer_id.clone(),
            },
        ));
    }

    fn inject_event(&mut self, source: PeerId, _: ConnectionId, event: InnerMessage) {
        trace!(
            "peer_service/connect_protocol/inject_node_event: new event {:?} received",
            event
        );

        match event {
            InnerMessage::Rx(m) => match m {
                ToNodeNetworkMsg::Relay { dst_id, data } => self.enqueue_event(ToNodeMsg::Relay {
                    src_id: source,
                    dst_id,
                    data,
                }),
                ToNodeNetworkMsg::Provide { key } => self.enqueue_event(ToNodeMsg::Provide(key)),
                ToNodeNetworkMsg::FindProviders { client_id, key } => {
                    self.enqueue_event(ToNodeMsg::FindProviders { client_id, key })
                }
                ToNodeNetworkMsg::Upgrade => {}
            },
            InnerMessage::Tx => {}
        }
    }

    // produces ToNodeMsg events
    event_polling!(poll, events, SwarmEventType);
}

/// Transmission between the OneShotHandler message type and the InNodeMessage message type.
#[derive(Debug)]
pub enum InnerMessage {
    /// Message has been received from a remote.
    Rx(ToNodeNetworkMsg),

    /// RelayMessage has been sent
    Tx,
}

impl From<ToNodeNetworkMsg> for InnerMessage {
    #[inline]
    fn from(in_node_message: ToNodeNetworkMsg) -> InnerMessage {
        InnerMessage::Rx(in_node_message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}
