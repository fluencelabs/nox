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

use async_trait::async_trait;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;

use avm_server::{CallResults, ParticleParameters};
use fluence_keypair::KeyPair;

use fluence_libp2p::PeerId;
use particle_protocol::Particle;

use crate::aqua_runtime::AquaRuntime;
use crate::{InterpretationStats, ParticleDataStore, ParticleEffects};

pub(super) type AVMRes<RT> = FutResult<Option<RT>, ParticleEffects, InterpretationStats>;

#[async_trait]
pub trait ParticleExecutor {
    type Output;
    type Particle;
    async fn execute(
        mut self,
        data_store: Arc<ParticleDataStore>,
        p: Self::Particle,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Output;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult<RT, Eff, Stats> {
    /// Return back AVM that just executed a particle and other reusable entities needed for execution
    pub runtime: RT,
    /// Outcome produced by particle execution
    pub effects: Eff,
    /// Performance stats
    pub stats: Stats,
}

#[async_trait]
impl<RT: AquaRuntime> ParticleExecutor for RT {
    type Output = AVMRes<RT>;
    type Particle = (Particle, CallResults);

    async fn execute(
        mut self,
        data_store: Arc<ParticleDataStore>,
        p: Self::Particle,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Output {
        let (particle, call_results) = p;
        let particle_id = particle.id.clone();

        let prev_data = data_store
            .read_data(particle_id.as_str(), current_peer_id.to_base58().as_str())
            .await;

        if let Ok(prev_data) = prev_data {
            tracing::trace!(target: "execution", particle_id = particle_id, "Executing particle");
            let prev_data_len = prev_data.len();
            let moved_prev_data = prev_data.clone();
            let moved_call_results = call_results.clone();
            let moved_particle_id = particle_id.clone();
            let avm_result = tokio::task::spawn_blocking(move || {
                let now = Instant::now();
                let memory_size_before = self.memory_stats().memory_size;
                let particle_params = ParticleParameters {
                    current_peer_id: Cow::Owned(current_peer_id.to_string()),
                    init_peer_id: Cow::Owned(particle.init_peer_id.to_string()),
                    // we use signature hex as particle id to prevent compromising of particle data store
                    particle_id: Cow::Owned(moved_particle_id),
                    timestamp: particle.timestamp,
                    ttl: particle.ttl,
                };
                let air = particle.script.clone();
                let current_data = particle.data.clone();
                let avm_outcome = self.call(
                    air,
                    current_data,
                    moved_prev_data,
                    particle_params.clone(),
                    moved_call_results,
                    &key_pair,
                );
                let memory_size_after = self.memory_stats().memory_size;

                let interpretation_time = now.elapsed();
                let new_data_len = avm_outcome.as_ref().map(|e| e.data.len()).ok();
                let memory_delta = memory_size_after - memory_size_before;
                let stats = InterpretationStats {
                    memory_delta,
                    interpretation_time,
                    new_data_len,
                    success: avm_outcome.is_ok(),
                };
                (avm_outcome, stats, particle, particle_params, self)
            });

            let avm_result = avm_result.await;
            match avm_result {
                Ok(avm_result) => {
                    let (avm_outcome, stats, particle, particle_params, vm) = avm_result;
                    let particle_id = particle.id;
                    match &avm_outcome {
                        Ok(outcome) => {
                            let len = outcome.data.len();

                            tracing::trace!(
                                target: "execution", particle_id = particle_id,
                                "Particle interpreted in {} [{} bytes => {} bytes]",
                                humantime::format_duration(stats.interpretation_time), prev_data_len, len
                            );

                            if data_store.detect_anomaly(
                                stats.interpretation_time,
                                stats.memory_delta,
                                outcome,
                            ) {
                                let anomaly_result = data_store
                                    .save_anomaly_data(
                                        particle.script.as_str(),
                                        &prev_data,
                                        &particle.data,
                                        &call_results,
                                        &particle_params,
                                        outcome,
                                        stats.interpretation_time,
                                        stats.memory_delta,
                                    )
                                    .await;
                                if let Err(err) = anomaly_result {
                                    tracing::warn!(
                                        particle_id = particle_id,
                                        "Could not save anomaly result {}",
                                        err
                                    )
                                }
                            }

                            let store_result = data_store
                                .store_data(
                                    &outcome.data,
                                    particle_id.as_str(),
                                    current_peer_id.to_base58().as_str(),
                                )
                                .await;
                            if let Err(err) = store_result {
                                tracing::warn!(
                                    particle_id = particle_id,
                                    "Could not save particle result {}",
                                    err
                                );
                                return FutResult {
                                    runtime: Some(vm),
                                    effects: ParticleEffects::empty(),
                                    stats: InterpretationStats::failed(),
                                };
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                particle_id = particle_id,
                                "Error executing particle: {}",
                                err
                            )
                        }
                    }

                    let effects = Self::into_effects(avm_outcome, particle_id);

                    FutResult {
                        runtime: Some(vm),
                        effects,
                        stats,
                    }
                }
                Err(err) => {
                    if err.is_cancelled() {
                        tracing::warn!(particle_id, "Particle task was cancelled");
                    } else {
                        tracing::error!(particle_id, "Particle task panic");
                    }
                    let stats = InterpretationStats::failed();
                    let effects = ParticleEffects::empty();
                    FutResult {
                        // We loose an AVM instance here
                        // But it will be recreated via VmPool
                        runtime: None,
                        effects,
                        stats,
                    }
                }
            }
        } else {
            FutResult {
                runtime: Some(self),
                effects: ParticleEffects::empty(),
                stats: InterpretationStats::failed(),
            }
        }
    }
}
