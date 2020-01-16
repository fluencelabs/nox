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

use crate::node_service::connect_protocol::messages::{InNodeMessage, OutNodeMessage};
use crate::node_service::events::OutNodeServiceEvent;
use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};
use log::trace;
use std::collections::VecDeque;
use std::marker::PhantomData;
use tokio::prelude::*;

pub struct NodeConnectProtocolBehaviour<Substream> {
    /// Queue of received network messages from connected nodes
    /// that need to be handled during polling.
    events: VecDeque<NetworkBehaviourAction<OutNodeMessage, OutNodeServiceEvent>>,

    /// Pin generic.
    marker: PhantomData<Substream>,
}

impl<Substream> NodeConnectProtocolBehaviour<Substream> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            marker: PhantomData,
        }
    }

    pub fn relay_message(&mut self, src: PeerId, dst: PeerId, message: Vec<u8>) {
        trace!(
            "node_service/connect_protocol/behaviour: relaying message {:?} to {:?}",
            message,
            dst
        );

        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: dst,
            event: OutNodeMessage::Relay {
                src: src.into_bytes(),
                data: message,
            },
        })
    }

    pub fn send_network_state(&mut self, dst: PeerId, state: Vec<PeerId>) {
        trace!(
            "node_service/connect_protocol/behaviour: sending network state {:?} to {:?}",
            state,
            dst
        );

        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: dst,
            event: OutNodeMessage::NetworkState {
                state: state
                    .iter()
                    .cloned()
                    .map(|p| p.into_bytes())
                    .collect::<Vec<Vec<u8>>>(),
            },
        })
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviour
    for NodeConnectProtocolBehaviour<Substream>
{
    type ProtocolsHandler = OneShotHandler<Substream, InNodeMessage, OutNodeMessage, InnerMessage>;
    type OutEvent = OutNodeServiceEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, node_id: PeerId, _cp: ConnectedPoint) {
        trace!(
            "node_service/connect_protocol/inject_connected: new node {:?} joined",
            node_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            OutNodeServiceEvent::NodeConnected {
                node_id: node_id.clone(),
            },
        ));
    }

    fn inject_disconnected(&mut self, node_id: &PeerId, _cp: ConnectedPoint) {
        trace!(
            "node_service/connect_protocol/inject_disconnected: node {:?} disconnected",
            node_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            OutNodeServiceEvent::NodeDisconnected {
                node_id: node_id.clone(),
            },
        ));
    }

    fn inject_node_event(&mut self, source: PeerId, event: InnerMessage) {
        trace!(
            "node_service/connect_protocol/inject_node_event: new event {:?} received",
            event
        );

        match event {
            InnerMessage::Rx(m) => match m {
                InNodeMessage::Relay { dst, data } => self.events.push_back(
                    NetworkBehaviourAction::GenerateEvent(OutNodeServiceEvent::Relay {
                        src: source,
                        dst: PeerId::from_bytes(dst).unwrap(),
                        data,
                    }),
                ),
                InNodeMessage::GetNetworkState => {
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        OutNodeServiceEvent::GetNetworkState { src: source },
                    ))
                }
            },
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
        if let Some(e) = self.events.pop_front() {
            trace!(
                "node_service/connect_protocol/behaviour/poll: event {:?} popped",
                e
            );
            return Async::Ready(e);
        };

        Async::NotReady
    }
}

/// Transmission between the OneShotHandler message type and the InNodeMessage message type.
#[derive(Debug)]
pub enum InnerMessage {
    /// Message has been received from a remote.
    Rx(InNodeMessage),

    /// RelayMessage has been sent
    Tx,
}

impl From<InNodeMessage> for InnerMessage {
    #[inline]
    fn from(in_node_message: InNodeMessage) -> InnerMessage {
        InnerMessage::Rx(in_node_message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}
