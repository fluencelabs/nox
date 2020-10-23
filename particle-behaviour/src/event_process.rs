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

use crate::clients::ConnectionKind;
use crate::ParticleBehaviour;

use particle_actors::PlumberEvent;
use particle_closures::BuiltinCommand;
use particle_dht::DHTEvent;

use libp2p::swarm::NetworkBehaviourAction;

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ParticleBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<DHTEvent> for ParticleBehaviour {
    fn inject_event(&mut self, event: DHTEvent) {
        match event {
            DHTEvent::Published(_) => {}
            DHTEvent::PublishFailed(_, _) => {}
            DHTEvent::Forward { target, particle } => self.forward_particle(target, particle),
            DHTEvent::DialPeer { peer_id, condition } => self
                .events
                .push_back(NetworkBehaviourAction::DialPeer { peer_id, condition }),
            DHTEvent::Resolved { key, value } => self.mailbox.resolve_complete(key, Ok(value)),
            DHTEvent::ResolveFailed { err } => {
                self.mailbox.resolve_complete(err.key, Err(err.kind))
            }
            DHTEvent::Neighborhood { key, value } => self.mailbox.got_neighborhood(key, value),
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PlumberEvent> for ParticleBehaviour {
    fn inject_event(&mut self, event: PlumberEvent) {
        match event {
            PlumberEvent::Forward { target, particle } => {
                let kind = self.connection_kind(&target);
                if let ConnectionKind::Direct = kind {
                    self.forward_particle(target, particle);
                } else {
                    self.dht.send_to(target, particle)
                }
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<BuiltinCommand> for ParticleBehaviour {
    fn inject_event(&mut self, cmd: BuiltinCommand) {
        match cmd {
            BuiltinCommand::DHTResolve(key) => self.dht.resolve(key),
            BuiltinCommand::DHTNeighborhood(key) => self.dht.get_neighborhood(key),
        }
    }
}
