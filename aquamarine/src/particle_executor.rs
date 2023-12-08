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

use std::borrow::Cow;
use std::time::Instant;

use avm_server::avm_runner::RawAVMOutcome;
use avm_server::{CallResults, ParticleParameters};
use fluence_keypair::KeyPair;

use fluence_libp2p::PeerId;
use particle_protocol::Particle;

use crate::aqua_runtime::AquaRuntime;
use crate::InterpretationStats;

pub(super) type AVMRes<RT> =
    FutResult<Option<RT>, Result<RawAVMOutcome, <RT as AquaRuntime>::Error>, InterpretationStats>;
pub trait ParticleExecutor {
    type Output;
    type Particle;
    fn execute(
        self,
        p: Self::Particle,
        prev_data: impl Into<Vec<u8>>,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Output;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult<RT, Out, Stats> {
    /// Return back AVM that just executed a particle and other reusable entities needed for execution
    pub runtime: RT,
    /// Outcome produced by particle execution
    pub outcome: Out,
    /// Performance stats
    pub stats: Stats,
}

impl<RT: AquaRuntime> ParticleExecutor for RT {
    type Output = AVMRes<RT>;
    type Particle = (Particle, CallResults);

    fn execute(
        mut self,
        p: Self::Particle,
        prev_data: impl Into<Vec<u8>>,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Output {
        let (particle, calls) = p;

        let now = Instant::now();
        tracing::trace!(target: "execution", particle_id = particle.id, "Executing particle");

        let particle_params = ParticleParameters {
            current_peer_id: Cow::Owned(current_peer_id.to_string()),
            init_peer_id: Cow::Owned(particle.init_peer_id.to_string()),
            // we use signature hex as particle id to prevent compromising of particle data store
            particle_id: Cow::Borrowed(&particle.id),
            timestamp: particle.timestamp,
            ttl: particle.ttl,
        };
        let avm_outcome = self.call(
            particle.script,
            particle.data,
            prev_data,
            particle_params,
            calls,
            &key_pair,
        );

        let call_time = now.elapsed();
        let new_data_len = avm_outcome.as_ref().map(|e| e.data.len()).ok();
        let stats = InterpretationStats {
            interpretation_time: call_time,
            new_data_len,
            success: avm_outcome.is_ok(),
        };

        FutResult {
            runtime: Some(self),
            outcome: avm_outcome,
            stats,
        }
    }
}
