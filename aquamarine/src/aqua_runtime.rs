/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use async_trait::async_trait;
use std::str::FromStr;
use std::{error::Error, task::Waker};

use avm_server::avm_runner::{AVMRunner, RawAVMOutcome};
use avm_server::{
    AVMMemoryStats, AVMRuntimeLimits, CallRequests, CallResults, ParticleParameters, RunnerError,
};
use fluence_keypair::KeyPair;
use libp2p::PeerId;
use marine_wasmtime_backend::WasmtimeWasmBackend;
use tracing::Level;

use crate::config::VmConfig;
use crate::error::{ExecutionError, FieldError};
use crate::particle_effects::ParticleEffects;

#[async_trait]
pub trait AquaRuntime: Sized + Send + 'static {
    type Config: Clone + Send + 'static;
    type Error: Error + Send + Sync + 'static;

    fn create_runtime(
        config: Self::Config,
        wasm_backend: WasmtimeWasmBackend,
        waker: Waker,
    ) -> Result<Self, Self::Error>;

    // TODO: move into_effects inside call
    fn into_effects(
        outcome: Result<RawAVMOutcome, Self::Error>,
        particle_id: String,
    ) -> ParticleEffects;

    async fn call(
        &mut self,
        air: impl Into<String> + Send,
        prev_data: impl Into<Vec<u8>> + Send,
        current_data: impl Into<Vec<u8>> + Send,
        particle_params: ParticleParameters<'_>,
        call_results: CallResults,
        key_pair: &KeyPair,
    ) -> Result<RawAVMOutcome, Self::Error>;

    /// Return current size of memory. Use only for diagnostics purposes.
    fn memory_stats(&self) -> AVMMemoryStats;
}

#[async_trait]
impl AquaRuntime for AVMRunner<WasmtimeWasmBackend> {
    type Config = VmConfig;
    type Error = RunnerError;

    /// Creates `AVM` in background (on blocking threadpool)
    fn create_runtime(
        config: Self::Config,
        backend: WasmtimeWasmBackend,
        waker: Waker,
    ) -> Result<Self, Self::Error> {
        let avm_runtime_limits = AVMRuntimeLimits::new(
            config.air_size_limit,
            config.particle_size_limit,
            config.call_result_size_limit,
            config.hard_limit_enabled,
        );
        let vm: AVMRunner<WasmtimeWasmBackend> =
            tokio::runtime::Handle::current().block_on(AVMRunner::new(
                config.air_interpreter,
                config.max_heap_size,
                avm_runtime_limits,
                i32::MAX,
                backend,
            ))?;
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

                if tracing::enabled!(Level::DEBUG) {
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
    async fn call(
        &mut self,
        air: impl Into<String> + Send,
        prev_data: impl Into<Vec<u8>> + Send,
        current_data: impl Into<Vec<u8>> + Send,
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
        .await
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
