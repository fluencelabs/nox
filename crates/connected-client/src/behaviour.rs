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
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::core::Endpoint;
use libp2p::identity::PublicKey;
use libp2p::swarm::ToSwarm::GenerateEvent;
use libp2p::swarm::{
    ConnectionDenied, ConnectionId, DialError, FromSwarm, THandler, THandlerInEvent,
    THandlerOutEvent,
};
use libp2p::{
    core::{connection::ConnectedPoint, Multiaddr},
    identify::{Behaviour as Identify, Config as IdentifyConfig},
    ping::{Behaviour as Ping, Config as PingConfig},
    swarm::{NetworkBehaviour, NotifyHandler, OneShotHandler, PollParameters, ToSwarm},
    PeerId,
};
use particle_protocol::{HandlerMessage, Particle, ProtocolConfig, PROTOCOL_NAME};

use crate::ClientEvent;

pub type SwarmEventType = ToSwarm<ClientEvent, THandlerInEvent<ClientBehaviour>>;

#[derive(NetworkBehaviour)]
pub struct FluenceClientBehaviour {
    client: ClientBehaviour,
    ping: Ping,
    identify: Identify,
}

impl FluenceClientBehaviour {
    pub fn new(protocol_config: ProtocolConfig, public_key: PublicKey) -> Self {
        let client = ClientBehaviour::new(protocol_config);
        let identify = Identify::new(IdentifyConfig::new(PROTOCOL_NAME.into(), public_key));
        let ping = Ping::new(
            PingConfig::new()
                .with_interval(Duration::from_secs(5))
                .with_timeout(Duration::from_secs(60)),
        );
        Self {
            client,
            ping,
            identify,
        }
    }

    pub fn call(&mut self, peer_id: PeerId, call: Particle) {
        self.client.events.push_back(ToSwarm::NotifyHandler {
            event: HandlerMessage::OutParticle(call, <_>::default()),
            handler: NotifyHandler::Any,
            peer_id,
        });

        self.client.wake();
    }
}

pub struct ClientBehaviour {
    protocol_config: ProtocolConfig,
    events: VecDeque<SwarmEventType>,
    reconnect: Option<BoxFuture<'static, Vec<Multiaddr>>>,
    waker: Option<Waker>,
}

impl ClientBehaviour {
    pub fn new(protocol_config: ProtocolConfig) -> Self {
        Self {
            protocol_config,
            events: VecDeque::default(),
            reconnect: None,
            waker: None,
        }
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref()
        }
    }

    fn on_connection_established(&mut self, peer_id: &PeerId, cp: &ConnectedPoint) {
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

        self.events
            .push_back(ToSwarm::GenerateEvent(ClientEvent::NewConnection {
                peer_id: *peer_id,
                multiaddr: multiaddr.clone(),
            }))
    }

    fn on_dial_failure(&mut self, peer_id: Option<PeerId>, error: &DialError) {
        log::warn!(
            "Failed to connect to {:?}: {:?}, reconnecting",
            peer_id,
            error
        );

        if let DialError::Transport(addresses) = error {
            let addresses = addresses.iter().map(|(a, _)| a.clone()).collect();
            self.reconnect = async move {
                // TODO: move timeout to config
                tokio::time::sleep(Duration::from_secs(1)).await;
                addresses
            }
            .boxed()
            .into();
        }
    }

    fn on_connection_closed(
        &mut self,
        peer_id: &PeerId,
        cp: &ConnectedPoint,
        remaining_established: usize,
    ) {
        if remaining_established != 0 {
            // not disconnected, we don't care
            return;
        }

        match cp {
            ConnectedPoint::Dialer { address, .. } => {
                let address = address.clone();
                log::warn!(
                    "Disconnected from {} @ {:?}, reconnecting",
                    peer_id,
                    address
                );
                self.events.push_front(SwarmEventType::Dial {
                    opts: address.into(),
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
}

impl NetworkBehaviour for ClientBehaviour {
    type ConnectionHandler = OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage>;

    type ToSwarm = ClientEvent;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer_id: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let oneshot_handler: OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage> =
            self.protocol_config.clone().into();

        Ok(oneshot_handler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let oneshot_handler: OneShotHandler<ProtocolConfig, HandlerMessage, HandlerMessage> =
            self.protocol_config.clone().into();
        Ok(oneshot_handler)
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_, Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(e) => {
                tracing::info!("ConnectionEstablished {:?}", e);
                self.on_connection_established(&e.peer_id, e.endpoint);
            }
            FromSwarm::ConnectionClosed(e) => {
                tracing::info!(
                    "ConnectionClosed {:?} {}",
                    e.remaining_established,
                    e.peer_id,
                );
                self.on_connection_closed(&e.peer_id, e.endpoint, e.remaining_established);
            }
            FromSwarm::AddressChange(_) => {}
            FromSwarm::DialFailure(e) => {
                tracing::info!("DialFailure {:?}", e);
                self.on_dial_failure(e.peer_id, e.error);
            }
            FromSwarm::ListenFailure(_) => {}
            FromSwarm::NewListener(_) => {}
            FromSwarm::NewListenAddr(_) => {}
            FromSwarm::ExpiredListenAddr(_) => {}
            FromSwarm::ListenerError(_) => {}
            FromSwarm::ListenerClosed(_) => {}
            FromSwarm::NewExternalAddrCandidate(_) => {}
            FromSwarm::ExternalAddrExpired(_) => {}
            FromSwarm::ExternalAddrConfirmed(_) => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _cid: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        use ClientEvent::Particle;

        match event {
            HandlerMessage::InParticle(particle) => {
                self.events.push_back(GenerateEvent(Particle {
                    particle,
                    sender: peer_id,
                }))
            }
            _ => {}
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<SwarmEventType> {
        self.waker = Some(cx.waker().clone());

        if let Some(Poll::Ready(addresses)) = self.reconnect.as_mut().map(|r| r.poll_unpin(cx)) {
            self.reconnect = None;
            for addr in addresses {
                self.events.push_front(ToSwarm::Dial { opts: addr.into() });
            }
        }

        if let Some(event) = self.events.pop_front() {
            tracing::info!("Connected client self.events.pop_front() {:?}", event);
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
