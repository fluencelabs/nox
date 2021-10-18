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

use avm_server::CallRequests;
use particle_protocol::Particle;

use libp2p::PeerId;

#[derive(Clone, Debug, Default)]
/// Effects produced by particle execution. Currently the only effect is that of sending particles.
pub struct ParticleEffects {
    /// Particle associated with these effects
    pub particle: Particle,
    /// Instruction to send particle to these peers
    pub next_peers: Vec<PeerId>,
    /// Instruction to execute host calls
    pub call_requests: CallRequests,
}

/// Network part of the [[ParticleEffects]. Can't be executed by Aquamarine layer,
/// thus delegated to outside.
pub struct NetworkEffects {
    pub particle: Particle,
    pub next_peers: Vec<PeerId>,
}
