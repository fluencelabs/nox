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

use super::network::NetworkBehaviour;
use crate::bootstrapper::BootstrapperEvent;
use libp2p::swarm::NetworkBehaviourEventProcess;

impl NetworkBehaviourEventProcess<BootstrapperEvent> for NetworkBehaviour {
    fn inject_event(&mut self, event: BootstrapperEvent) {
        log::debug!(target: "fix_bootstrapper", "got bootstrapper event: {:?}", event);
        // todo!("implement bootstrapper as a part of kademlia?")
        // TODO: do not reconnect to boostraps all the time, make it stop after a few minutes after node was started
        //       In other words, reconnect first 5 minutes or so, then stop. No reason to treat bootstrap nodes in a special way anymore.
        // match event {
        //     BootstrapperEvent::RunBootstrap => {
        //         log::debug!("Running bootstrap procedure");
        //         self.bootstrap()
        //     }
        //     BootstrapperEvent::ReconnectToBootstrap { multiaddr, error } => {
        //         log::debug!(
        //             "Bootstrap disconnected {} {}, reconnecting",
        //             multiaddr,
        //             error.unwrap_or_default()
        //         );
        //         self.dial(multiaddr);
        //     }
        // }
    }
}
