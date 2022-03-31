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

use std::collections::VecDeque;
use std::error::Error;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::DialError;
use libp2p::{
    core::{
        connection::{ConnectedPoint, ConnectionId},
        either::EitherOutput,
        Multiaddr,
    },
    ping::{Ping, PingConfig, PingResult},
    swarm::{
        IntoConnectionHandler, IntoConnectionHandlerSelect, NetworkBehaviour,
        NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
    },
    PeerId,
};

use fluence_libp2p::generate_swarm_event_type;
use particle_protocol::{HandlerMessage, Particle, ProtocolConfig};

use crate::ClientEvent;

pub type SwarmEventType =
    NetworkBehaviourAction<ClientEvent, <ClientBehaviour as NetworkBehaviour>::ConnectionHandler>;

pub struct ClientBehaviour {
    protocol_config: ProtocolConfig,
    events: VecDeque<SwarmEventType>,
    ping: Ping,
    reconnect: Option<BoxFuture<'static, Vec<Multiaddr>>>,
    waker: Option<Waker>,
}

impl ClientBehaviour {
    pub fn new(protocol_config: ProtocolConfig) -> Self {
        let ping = Ping::new(PingConfig::new().with_keep_alive(true));
        Self {
            protocol_config,
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
    type ConnectionHandler = IntoConnectionHandlerSelect<
        <OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage> as IntoConnectionHandler>::Handler,
        <Ping as NetworkBehaviour>::ConnectionHandler,
    >;

    type OutEvent = ClientEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        IntoConnectionHandler::select(self.protocol_config.clone().into(), self.ping.new_handler())
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
        _failed_addresses: Option<&Vec<Multiaddr>>,
        _: usize,
    ) {
        let multiaddr = match cp {
            ConnectedPoint::Dialer { address, .. } => address,
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
                peer_id: *peer_id,
                multiaddr: multiaddr.clone(),
            },
        ))
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
        _: <Self::ConnectionHandler as IntoConnectionHandler>::Handler,
        remaining_established: usize,
    ) {
        if remaining_established != 0 {
            // not disconnected, we don't care
            return;
        }

        match cp {
            ConnectedPoint::Dialer { address, .. } => {
                log::warn!(
                    "Disconnected from {} @ {:?}, reconnecting",
                    peer_id,
                    address
                );
                let handler = self.new_handler();
                self.events.push_back(NetworkBehaviourAction::Dial {
                    opts: address.clone().into(),
                    handler,
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

    fn inject_dial_failure(
        &mut self,
        peer_id: Option<PeerId>,
        _handler: Self::ConnectionHandler,
        error: &DialError,
    ) {
        log::warn!(
            "Failed to connect to {:?}: {:?}, reconnecting",
            peer_id,
            error
        );

        if let DialError::Transport(addresses) = error {
            let addresses = addresses.iter().map(|(a, _)| a.clone()).collect();
            self.reconnect = async move {
                // TODO: move timeout to config
                async_std::task::sleep(Duration::from_secs(1)).await;
                addresses
            }
            .boxed()
            .into();
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        // just polling it to the end
        while self.ping.poll(cx, params).is_ready() {}

        if let Some(Poll::Ready(addresses)) = self.reconnect.as_mut().map(|r| r.poll_unpin(cx)) {
            self.reconnect = None;
            for addr in addresses {
                let handler = self.new_handler();
                self.events.push_back(NetworkBehaviourAction::Dial {
                    opts: addr.into(),
                    handler,
                });
            }
        }

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
