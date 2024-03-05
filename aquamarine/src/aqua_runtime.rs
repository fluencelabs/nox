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

use std::str::FromStr;
use std::{error::Error, task::Waker};

use avm_server::avm_runner::{AVMRunner, RawAVMOutcome};
use avm_server::{
    AVMMemoryStats, AVMRuntimeLimits, CallRequests, CallResults, ParticleParameters, RunnerError,
};
use fluence_keypair::KeyPair;
use libp2p::PeerId;
use tracing::Level;

use crate::config::VmConfig;
use crate::error::{ExecutionError, FieldError};
use crate::particle_effects::ParticleEffects;

pub trait AquaRuntime: Sized + Send + 'static {
    type Config: Clone + Send + 'static;
    type Error: Error + Send + Sync + 'static;

    fn create_runtime(config: Self::Config, waker: Waker) -> Result<Self, Self::Error>;

    // TODO: move into_effects inside call
    fn into_effects(
        outcome: Result<RawAVMOutcome, Self::Error>,
        particle_id: String,
    ) -> ParticleEffects;

    fn call(
        &mut self,
        air: impl Into<String>,
        prev_data: impl Into<Vec<u8>>,
        current_data: impl Into<Vec<u8>>,
        particle_params: ParticleParameters<'_>,
        call_results: CallResults,
        key_pair: &KeyPair,
    ) -> Result<RawAVMOutcome, Self::Error>;

    /// Return current size of memory. Use only for diagnostics purposes.
    fn memory_stats(&self) -> AVMMemoryStats;
}

impl AquaRuntime for AVMRunner {
    type Config = VmConfig;
    type Error = RunnerError;

    /// Creates `AVM` in background (on blocking threadpool)
    fn create_runtime(config: Self::Config, waker: Waker) -> Result<Self, Self::Error> {
        let avm_runtime_limits = AVMRuntimeLimits::new(
            config.air_size_limit,
            config.particle_size_limit,
            config.call_result_size_limit,
            config.hard_limit_enabled,
        );
        let vm: AVMRunner = AVMRunner::new(
            config.air_interpreter,
            config.max_heap_size,
            avm_runtime_limits,
            i32::MAX,
        )?;
        waker.wake();
        Ok(vm)
    }

    fn into_effects(
        outcome: Result<RawAVMOutcome, Self::Error>,
        particle_id: String,
    ) -> ParticleEffects {
        match parse_outcome(outcome) {
            Ok((new_data, peers, calls)) if !peers.is_empty() || !calls.is_empty() => {
                #[rustfmt::skip]
                tracing::debug!(
                    target: "execution",
                    particle_id,
                    "Particle executed: {} call requests, {} next peers", calls.len(), peers.len()
                );

                ParticleEffects {
                    next_peers: peers,
                    call_requests: calls,
                    new_data,
                }
            }
            Ok((data, ..)) => {
                tracing::debug!(
                    target: "execution",
                    particle_id,
                    "Executed particle, next_peer_pks is empty, no call requests. Nothing to do.",
                );

                if tracing::enabled!(target: "execution", Level::DEBUG) {
                    let data = String::from_utf8_lossy(data.as_slice());
                    tracing::debug!(particle_id, "particle next_peer_pks = [], data: {}", data);
                }

                ParticleEffects::empty()
            }
            Err(ExecutionError::AquamarineError(err)) => {
                tracing::warn!(target: "execution", particle_id, "Error executing particle: {}", err);
                ParticleEffects::empty()
            }
            Err(err @ ExecutionError::InvalidResultField { .. }) => {
                tracing::warn!(target: "execution", particle_id, "Error parsing outcome for particle: {}", err);
                ParticleEffects::empty()
            }
        }
    }

    #[inline]
    fn call(
        &mut self,
        air: impl Into<String>,
        prev_data: impl Into<Vec<u8>>,
        current_data: impl Into<Vec<u8>>,
        particle_params: ParticleParameters<'_>,
        call_results: CallResults,
        key_pair: &KeyPair,
    ) -> Result<RawAVMOutcome, Self::Error> {
        AVMRunner::call(
            self,
            air,
            prev_data,
            current_data,
            particle_params.init_peer_id,
            particle_params.timestamp,
            particle_params.ttl,
            particle_params.current_peer_id,
            call_results,
            key_pair,
            particle_params.particle_id.to_string(),
        )
    }

    fn memory_stats(&self) -> AVMMemoryStats {
        self.memory_stats()
    }
}

pub fn parse_outcome(
    outcome: Result<RawAVMOutcome, RunnerError>,
) -> Result<(Vec<u8>, Vec<PeerId>, CallRequests), ExecutionError> {
    let outcome = outcome.map_err(ExecutionError::AquamarineError)?;

    let peer_ids = outcome
        .next_peer_pks
        .into_iter()
        .map(|id| {
            parse_peer_id(id.as_str()).map_err(|error| ExecutionError::InvalidResultField {
                field: "next_peer_pks[..]",
                error,
            })
        })
        .collect::<Result<_, ExecutionError>>()?;

    Ok((outcome.data, peer_ids, outcome.call_requests))
}

fn parse_peer_id(s: &str) -> Result<PeerId, FieldError> {
    PeerId::from_str(s).map_err(|err| FieldError::InvalidPeerId {
        peer_id: s.to_string(),
        err: err.to_string(),
    })
}
