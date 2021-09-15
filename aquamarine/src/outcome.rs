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

#[derive(Clone, Debug)]
pub struct SendParticle {
    pub particle: Particle,
    pub target: PeerId,
}

#[derive(Clone, Debug, Default)]
/// Effects produced by particle execution. Currently the only effect is that of sending particles.
pub struct StepperEffects {
    /// Particles that either correspond to `next_peer_pks` or represent an error being sent back to `init_peer_id`
    pub particles: Vec<SendParticle>,
    pub call_requests: CallRequests,
}
