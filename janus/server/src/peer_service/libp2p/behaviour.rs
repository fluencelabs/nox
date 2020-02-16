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

use crate::peer_service::libp2p::connect_protocol::behaviour::PeerConnectProtocolBehaviour;
use crate::peer_service::libp2p::notifications::OutPeerNotification;
use crate::event_polling;
use libp2p::core::either::EitherOutput;
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess};
use libp2p::{NetworkBehaviour, PeerId};
use log::debug;
use std::collections::VecDeque;

/// This type is constructed inside NetworkBehaviour proc macro and represents the InEvent type
/// parameter of NetworkBehaviourAction. Should be regenerated each time a set of behaviours
/// of the PeerServiceBehaviour is changed.
type PeerServiceBehaviourInEvent = EitherOutput<EitherOutput<
    <<<libp2p::ping::Ping as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent,
    <<<libp2p::identify::Identify as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>,
    <<<PeerConnectProtocolBehaviour as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler as libp2p::swarm::protocols_handler::IntoProtocolsHandler>::Handler as libp2p::swarm::protocols_handler::ProtocolsHandler>::InEvent>;

#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", out_event = "OutPeerNotification")]
pub struct PeerServiceBehaviour {
    ping: Ping,
    identity: Identify,
    node_connect_protocol: PeerConnectProtocolBehaviour,

    #[behaviour(ignore)]
    events: VecDeque<NetworkBehaviourAction<PeerServiceBehaviourInEvent, OutPeerNotification>>,
}

impl NetworkBehaviourEventProcess<OutPeerNotification> for PeerServiceBehaviour {
    fn inject_event(&mut self, event: OutPeerNotification) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));
    }
}

impl NetworkBehaviourEventProcess<PingEvent> for PeerServiceBehaviour {
    fn inject_event(&mut self, event: PingEvent) {
        if event.result.is_err() {
            debug!("ping failed with {:?}", event);
        }
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for PeerServiceBehaviour {
    fn inject_event(&mut self, _event: IdentifyEvent) {}
}

impl PeerServiceBehaviour {
    pub fn new(_local_peer_id: &PeerId, local_public_key: PublicKey) -> Self {
        let ping = Ping::new(
            PingConfig::new()
                .with_max_failures(unsafe { core::num::NonZeroU32::new_unchecked(5) })
                .with_keep_alive(true),
        );
        let identity = Identify::new("1.0.0".into(), "1.0.0".into(), local_public_key);
        let node_connect_protocol = PeerConnectProtocolBehaviour::new();

        Self {
            ping,
            identity,
            node_connect_protocol,
            events: VecDeque::new(),
        }
    }

    pub fn relay_message(&mut self, src: PeerId, dst: PeerId, message: Vec<u8>) {
        self.node_connect_protocol.relay_message(src, dst, message);
    }

    pub fn send_network_state(&mut self, dst: PeerId, state: Vec<PeerId>) {
        self.node_connect_protocol.send_network_state(dst, state);
    }

    #[allow(dead_code)]
    pub fn exit(&mut self) {
        unimplemented!("need to decide how exactly NodeDisconnect message will be sent");
    }

    // produces OutPeerNotification events
    event_polling!(
        custom_poll,
        events,
        NetworkBehaviourAction<PeerServiceBehaviourInEvent, OutPeerNotification>
    );
}
