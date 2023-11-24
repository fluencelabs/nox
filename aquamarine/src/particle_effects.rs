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
use particle_protocol::{ExtendedParticle};
use std::time::Duration;

use libp2p::PeerId;

#[derive(Clone, Debug)]
/// Effects produced by particle execution. Currently the only effect is that of sending particles.
pub struct ParticleEffects {
    /// New particle data
    pub new_data: Vec<u8>,
    /// Instruction to send particle to these peers
    pub next_peers: Vec<PeerId>,
    /// Instruction to execute host calls
    pub call_requests: CallRequests,
}

impl ParticleEffects {
    pub fn empty() -> Self {
        Self {
            new_data: vec![],
            next_peers: vec![],
            call_requests: <_>::default(),
        }
    }
}

#[derive(Clone, Debug)]
/// Performance stats about particle's interpretation
pub struct InterpretationStats {
    pub interpretation_time: Duration,
    pub call_time: Duration,
    pub new_data_len: Option<usize>,
    pub success: bool,
}

/// Routing part of the [[ParticleEffects].
/// Instruct to send particle to either virtual or remote peers.
#[derive(Clone, Debug)]
pub struct RoutingEffects {
    pub particle: ExtendedParticle,
    pub next_peers: Vec<PeerId>,
}
