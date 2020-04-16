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

use crate::node_service::function::{FunctionRouter, SwarmEventType};
use faas_api::ProtocolMessage;

use log::{debug, trace};
use std::error::Error;
use std::task::{Context, Poll};

use libp2p::{
    core::{
        connection::{ConnectedPoint, ConnectionId, ListenerId},
        either::EitherOutput,
        Multiaddr,
    },
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent},
    swarm::{
        IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, OneShotHandler, PollParameters, ProtocolsHandler,
    },
    PeerId,
};

impl NetworkBehaviour for FunctionRouter {
    // type ProtocolsHandler = OneShotHandler<ProtocolMessage, ProtocolMessage, ProtocolMessage>;

    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        OneShotHandler<ProtocolMessage, ProtocolMessage, ProtocolMessage>,
        <Kademlia<MemoryStore> as NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(Default::default(), self.kademlia.new_handler())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        log::info!("addresses_of_peer {}", peer_id.to_base58());
        self.kademlia.addresses_of_peer(peer_id)
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        log::debug!("inject_connected {}", peer_id.to_base58());
        self.connected(peer_id.clone());
        self.kademlia.inject_connected(peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        log::debug!("inject_disconnected {}", peer_id.to_base58());
        self.disconnected(peer_id);
        self.kademlia.inject_disconnected(peer_id);
    }

    fn inject_connection_established(&mut self, p: &PeerId, i: &ConnectionId, c: &ConnectedPoint) {
        log::debug!("connection_established {} {:?} {:?}", p.to_base58(), i, c);
        self.kademlia.inject_connection_established(p, i, c);
    }

    fn inject_connection_closed(&mut self, p: &PeerId, i: &ConnectionId, c: &ConnectedPoint) {
        log::debug!("connection_closed {} {:?} {:?}", p.to_base58(), i, c);
        self.kademlia.inject_connection_closed(p, i, c)
    }

    fn inject_event(
        &mut self,
        source: PeerId,
        connection_id: ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        use EitherOutput::{First, Second};

        match event {
            First(ProtocolMessage::FunctionCall(call)) => {
                log::info!("FunctionCall! from {} {:?}", source.to_base58(), call);
                self.call(call)
            }
            Second(kademlia_event) => {
                trace!("Kademlia: {:?}", kademlia_event);
                self.kademlia
                    .inject_event(source, connection_id, kademlia_event)
            }
            _ => {}
        }
    }

    fn inject_addr_reach_failure(&mut self, p: Option<&PeerId>, a: &Multiaddr, e: &dyn Error) {
        self.kademlia.inject_addr_reach_failure(p, a, e);
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        // TODO: clear connected_peers on inject_listener_closed?
        self.disconnected(peer_id);
        self.kademlia.inject_dial_failure(peer_id);
    }

    fn inject_new_listen_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_new_listen_addr(a)
    }

    fn inject_expired_listen_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_expired_listen_addr(a)
    }

    fn inject_new_external_addr(&mut self, a: &Multiaddr) {
        self.kademlia.inject_new_external_addr(a)
    }

    fn inject_listener_error(&mut self, i: ListenerId, e: &(dyn Error + 'static)) {
        self.kademlia.inject_listener_error(i, e)
    }

    fn inject_listener_closed(&mut self, i: ListenerId, reason: Result<(), &std::io::Error>) {
        self.kademlia.inject_listener_closed(i, reason)
    }

    fn poll(&mut self, cx: &mut Context, params: &mut impl PollParameters) -> Poll<SwarmEventType> {
        use NetworkBehaviourAction::*;
        use NetworkBehaviourEventProcess as NBEP;

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // TODO: would be nice to generate that with macro
        loop {
            match self.kademlia.poll(cx, params) {
                Poll::Ready(GenerateEvent(event)) => NBEP::inject_event(self, event),
                Poll::Ready(NotifyHandler {
                    peer_id,
                    event,
                    handler,
                }) => {
                    return Poll::Ready(NotifyHandler {
                        peer_id,
                        event: EitherOutput::Second(event),
                        handler,
                    })
                }
                Poll::Ready(DialAddress { address }) => {
                    return Poll::Ready(DialAddress { address })
                }
                Poll::Ready(ReportObservedAddr { address }) => {
                    return Poll::Ready(ReportObservedAddr { address })
                }
                Poll::Ready(DialPeer { peer_id, condition }) => {
                    return Poll::Ready(DialPeer { peer_id, condition })
                }
                Poll::Pending => break,
            }
        }

        Poll::Pending
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<KademliaEvent> for FunctionRouter {
    fn inject_event(&mut self, event: KademliaEvent) {
        use libp2p::kad::{GetProvidersError, GetProvidersOk};
        use KademliaEvent::GetProvidersResult;

        debug!("Kademlia inject: {:?}", event);

        match event {
            GetProvidersResult(Ok(GetProvidersOk { key, providers, .. })) => {
                self.providers_found(key, providers)
            }
            GetProvidersResult(Err(GetProvidersError::Timeout { key, providers, .. })) => {
                println!(
                    "GetProviders for {} timed out with {} providers",
                    bs58::encode(key.as_ref()).into_string(),
                    providers.len()
                );
                self.providers_found(key, providers)
            }
            _ => {}
        };
    }
}
