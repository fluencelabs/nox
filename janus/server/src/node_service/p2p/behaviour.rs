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

use libp2p::identify::Identify;
use libp2p::identity::PublicKey;

use libp2p::ping::{Ping, PingConfig};

use libp2p::PeerId;
use log::trace;

use crate::event_polling;
use crate::generate_swarm_event_type;

use crate::node_service::relay::KademliaRelay;
use crate::node_service::relay::Relay;
use crate::node_service::relay::RelayEvent;

mod identity;
mod ping;
mod relay;

type SwarmEventType = generate_swarm_event_type!(NodeServiceBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", out_event = "RelayEvent")]
pub struct NodeServiceBehaviour {
    ping: Ping,
    relay: KademliaRelay,
    identity: Identify,

    /// Contains events that need to be propagate to external caller.
    #[behaviour(ignore)]
    events: VecDeque<SwarmEventType>,
}

impl NodeServiceBehaviour {
    pub fn new(local_peer_id: PeerId, local_public_key: PublicKey) -> Self {
        let ping = Ping::new(
            PingConfig::new()
                .with_max_failures(unsafe { std::num::NonZeroU32::new_unchecked(10) })
                .with_keep_alive(true),
        );
        let relay = KademliaRelay::new(local_peer_id);
        let identity = Identify::new("/janus/p2p/1.0.0".into(), "0.1.0".into(), local_public_key);

        Self {
            ping,
            relay,
            identity,
            events: VecDeque::new(),
        }
    }

    /// Bootstraps the node. Currently, tells Kademlia to run bootstrapping lookup.
    pub fn bootstrap(&mut self) {
        self.relay.bootstrap();
    }

    pub fn add_local_peer(&mut self, peer_id: PeerId) {
        trace!(
            "node_service/p2p/behaviour: add connected peer {:?}",
            peer_id
        );

        self.relay.add_local_peer(peer_id);
    }

    pub fn remove_local_peer(&mut self, peer_id: PeerId) {
        trace!(
            "node_service/p2p/behaviour: remove connected peer {:?}",
            peer_id
        );

        self.relay.remove_local_peer(&peer_id);
    }

    pub fn relay(&mut self, relay_message: RelayEvent) {
        self.relay.relay(relay_message);
    }

    #[allow(dead_code)]
    pub fn exit(&mut self) {
        unimplemented!("need to decide how exactly NodeDisconnect message will be sent");
    }

    // produces RelayEvent
    event_polling!(custom_poll, events, SwarmEventType);
}
