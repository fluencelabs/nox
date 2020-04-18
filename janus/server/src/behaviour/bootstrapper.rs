/*
 * Copyright 2019 Fluence Labs Limited
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

use super::server_behaviour::ServerBehaviour;
use crate::BootstrapperEvent;
use libp2p::swarm::NetworkBehaviourEventProcess;

impl NetworkBehaviourEventProcess<BootstrapperEvent> for ServerBehaviour {
    fn inject_event(&mut self, event: BootstrapperEvent) {
        match event {
            BootstrapperEvent::BootstrapConnected { peer_id, .. } => {
                log::debug!(
                    "Bootstrap connected {}, triggering bootstrap procedure",
                    peer_id.to_base58()
                );
                self.bootstrap()
            }
            BootstrapperEvent::BootstrapDisconnected { peer_id, multiaddr } => {
                log::info!(
                    "Bootstrap disconnected {}, reconnecting",
                    peer_id.to_base58()
                );
                self.dial(multiaddr);
                self.dial_peer(peer_id);
            }
            BootstrapperEvent::ReachFailure {
                multiaddr, error, ..
            } => {
                log::warn!(
                    "Failed to reach bootstrap at {:?}: {}, reconnecting",
                    &multiaddr,
                    error
                );
                self.dial(multiaddr);
            }
        }
    }
}
