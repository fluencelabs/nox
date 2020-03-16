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

use crate::peer_service::connect_protocol::behaviour::PeerConnectBehaviour;
use crate::peer_service::messages::ToNodeMsg;
use crate::{event_polling, generate_swarm_event_type};

use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::identity::PublicKey;
use libp2p::ping::{handler::PingConfig, Ping, PingEvent};
use libp2p::swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess};
use libp2p::{NetworkBehaviour, PeerId};
use log::debug;

use parity_multiaddr::Multiaddr;
use parity_multihash::Multihash;
use std::collections::VecDeque;

type SwarmEventType = generate_swarm_event_type!(PeerServiceBehaviour);

#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", out_event = "ToNodeMsg")]
pub struct PeerServiceBehaviour {
    ping: Ping,
    identity: Identify,
    peers: PeerConnectBehaviour,

    #[behaviour(ignore)]
    events: VecDeque<SwarmEventType>,
}

impl NetworkBehaviourEventProcess<ToNodeMsg> for PeerServiceBehaviour {
    fn inject_event(&mut self, event: ToNodeMsg) {
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
        let node_connect_protocol = PeerConnectBehaviour::new();

        Self {
            ping,
            identity,
            peers: node_connect_protocol,
            events: VecDeque::new(),
        }
    }

    pub fn deliver_data(&mut self, src: PeerId, dst: PeerId, data: Vec<u8>) {
        self.peers.deliver_data(src, dst, data);
    }

    pub fn deliver_providers(
        &mut self,
        client_id: PeerId,
        key: Multihash,
        providers: Vec<(Multiaddr, PeerId)>,
    ) {
        self.peers.deliver_providers(client_id, key, providers)
    }

    #[allow(dead_code)]
    pub fn exit(&mut self) {
        unimplemented!("need to decide how exactly NodeDisconnect message will be sent");
    }

    // produces ToNodeMsg events
    event_polling!(custom_poll, events, SwarmEventType);
}
