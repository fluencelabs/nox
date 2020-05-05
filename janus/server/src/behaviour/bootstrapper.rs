/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
