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

use crate::node_service::p2p::behaviour::NodeServiceBehaviour;
use crate::node_service::relay::Relay;
use libp2p::identify::IdentifyEvent;
use libp2p::identity::PublicKey;
use libp2p::swarm::NetworkBehaviourEventProcess;

/// Network address information is exchanged via Identify protocol.
/// That information is passed to relay, so nodes know each other's addresses
impl NetworkBehaviourEventProcess<IdentifyEvent> for NodeServiceBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info, .. } => {
                if let PublicKey::Ed25519(public_key) = info.public_key {
                    self.relay
                        .add_node_addresses(&peer_id, info.listen_addrs, public_key)
                } else {
                    eprintln!(
                        "Unable to add node {}, public key {:?} is unsupported. Only ed25519 is supported.",
                        peer_id.to_base58(), info.public_key
                    );
                }
            }

            // TODO: handle error?
            IdentifyEvent::Error { .. } => {}

            // We don't care about Sent identification info
            IdentifyEvent::Sent { .. } => {}
        }
    }
}
