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

use std::task::{Context, Poll};

use libp2p::core::connection::ConnectionId;
use libp2p::core::either::EitherOutput;
use libp2p::core::ConnectedPoint;
use libp2p::core::Multiaddr;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::Kademlia;
use libp2p::kad::KademliaEvent;
use libp2p::swarm::IntoProtocolsHandler;
use libp2p::swarm::IntoProtocolsHandlerSelect;
use libp2p::swarm::NetworkBehaviour;
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::swarm::OneShotHandler;
use libp2p::swarm::PollParameters;
use libp2p::swarm::ProtocolsHandler;
use libp2p::PeerId;
use log::{debug, trace};

use crate::node_service::relay::kademlia::{KademliaRelay, SwarmEventType};
use crate::node_service::relay::{
    kademlia::events::InnerMessage, messages::RelayMessage, relay::Relay,
};
use crate::peer_service::messages::ToPeerMsg;

impl NetworkBehaviour for KademliaRelay {
    type ProtocolsHandler = IntoProtocolsHandlerSelect<
        OneShotHandler<RelayMessage, RelayMessage, InnerMessage>,
        <Kademlia<MemoryStore> as NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = ToPeerMsg;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(Default::default(), self.kademlia.new_handler())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.kademlia.addresses_of_peer(peer_id)
    }

    fn inject_connected(&mut self, peer_id: PeerId, cp: ConnectedPoint) {
        self.kademlia.inject_connected(peer_id, cp);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, cp: ConnectedPoint) {
        self.kademlia.inject_disconnected(peer_id, cp);
    }

    fn inject_event(
        &mut self,
        source: PeerId,
        connection_id: ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        use EitherOutput::{First, Second};

        match event {
            First(InnerMessage::Relay(relay)) => self.relay(relay),
            Second(kademlia_event) => {
                trace!("Kademlia: {:?}", kademlia_event);
                self.kademlia
                    .inject_event(source, connection_id, kademlia_event)
            }
            _ => {}
        }
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
                Poll::Ready(DialPeer { peer_id }) => return Poll::Ready(DialPeer { peer_id }),
                Poll::Pending => break,
            }
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for KademliaRelay {
    fn inject_event(&mut self, event: KademliaEvent) {
        use itertools::Itertools;
        use libp2p::kad::{GetProvidersError, GetProvidersOk};
        use KademliaEvent::GetProvidersResult;

        debug!("Kademlia inject: {:?}", event);

        let mut complete_providers = |key, providers: Vec<PeerId>| {
            let providers = providers.into_iter().unique().collect::<Vec<_>>(); //dedup
            self.providers_found(key, providers)
        };

        match event {
            GetProvidersResult(Ok(GetProvidersOk { key, providers, .. })) => {
                complete_providers(key, providers)
            }
            GetProvidersResult(Err(GetProvidersError::Timeout { key, providers, .. })) => {
                println!(
                    "GetProviders for {} timed out with {} providers",
                    bs58::encode(key.as_ref()).into_string(),
                    providers.len()
                );
                complete_providers(key, providers)
            }
            _ => {}
        };
    }
}
