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

use crate::ClientEvent;
use failure::_core::task::{Context, Poll, Waker};
use failure::_core::time::Duration;
use fluence_libp2p::generate_swarm_event_type;
use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::core::connection::{ConnectedPoint, ConnectionId};
use libp2p::core::either::EitherOutput;
use libp2p::core::Multiaddr;
use libp2p::ping::{Ping, PingConfig, PingResult};
use libp2p::swarm::{
    IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
    NotifyHandler, OneShotHandler, PollParameters,
};
use libp2p::PeerId;
use particle_protocol::{HandlerMessage, Particle, ProtocolConfig};
use std::collections::VecDeque;
use std::error::Error;

pub type SwarmEventType = generate_swarm_event_type!(ClientBehaviour);

pub struct ClientBehaviour {
    events: VecDeque<SwarmEventType>,
    ping: Ping,
    reconnect: Option<BoxFuture<'static, Multiaddr>>,
    waker: Option<Waker>,
}

impl ClientBehaviour {
    pub fn new() -> Self {
        let ping = Ping::new(PingConfig::new().with_keep_alive(true));
        Self {
            events: VecDeque::default(),
            ping,
            reconnect: None,
            waker: None,
        }
    }

    pub fn call(&mut self, peer_id: PeerId, call: Particle) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                event: EitherOutput::First(HandlerMessage::OutParticle(call, <_>::default())),
                handler: NotifyHandler::Any,
                peer_id,
            });

        self.wake();
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref()
        }
    }
}

impl NetworkBehaviour for ClientBehaviour {
    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        <OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage> as IntoProtocolsHandler>::Handler,
        <Ping as NetworkBehaviour>::ProtocolsHandler,
    >;

    type OutEvent = ClientEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        let protocol_config = ProtocolConfig::default();

        IntoProtocolsHandler::select(protocol_config.into(), self.ping.new_handler())
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let multiaddr = match cp {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener {
                send_back_addr,
                local_addr,
            } => {
                log::warn!(
                    "Someone connected to the client at {:?}. That's strange. {} @ {:?}",
                    local_addr,
                    peer_id,
                    send_back_addr
                );
                send_back_addr
            }
        };

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            ClientEvent::NewConnection {
                peer_id: peer_id.clone(),
                multiaddr: multiaddr.clone(),
            },
        ))
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        match cp {
            ConnectedPoint::Dialer { address } => {
                log::warn!(
                    "Disconnected from {} @ {:?}, reconnecting",
                    peer_id,
                    address
                );
                self.events.push_back(NetworkBehaviourAction::DialAddress {
                    address: address.clone(),
                });
            }
            ConnectedPoint::Listener {
                send_back_addr,
                local_addr,
            } => {
                log::warn!(
                    "Peer {} @ {:?} disconnected, was connected to {:?}, won't reconnect",
                    peer_id,
                    send_back_addr,
                    local_addr
                );
            }
        }
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        cid: ConnectionId,
        event: EitherOutput<HandlerMessage, PingResult>,
    ) {
        use ClientEvent::Particle;
        use EitherOutput::*;
        use NetworkBehaviourAction::GenerateEvent;

        match event {
            First(HandlerMessage::InParticle(particle)) => {
                self.events.push_back(GenerateEvent(Particle {
                    particle,
                    sender: peer_id,
                }))
            }
            Second(ping) => self.ping.inject_event(peer_id, cid, ping),
            First(_) => {}
        }
    }

    fn inject_addr_reach_failure(
        &mut self,
        _: Option<&PeerId>,
        addr: &Multiaddr,
        error: &dyn Error,
    ) {
        log::warn!("Failed to connect to {:?}: {:?}, reconnecting", addr, error);
        let address = addr.clone();
        self.reconnect = async move {
            // TODO: move timeout to config
            async_std::task::sleep(Duration::from_secs(1)).await;
            address
        }
        .boxed()
        .into();
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        // just polling it to the end
        while let Poll::Ready(_) = self.ping.poll(cx, params) {}

        if let Some(Poll::Ready(address)) = self.reconnect.as_mut().map(|r| r.poll_unpin(cx)) {
            self.reconnect = None;
            self.events
                .push_back(NetworkBehaviourAction::DialAddress { address });
        }

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
