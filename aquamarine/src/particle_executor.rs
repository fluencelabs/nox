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
        let particle_id = particle.id;

        let prev_data = data_store
            .read_data(particle_id.as_str(), current_peer_id.to_base58().as_str())
            .await;

        if let Ok(prev_data) = prev_data {
            let now = Instant::now();
            let memory_size_before = self.memory_stats().memory_size;
            tracing::trace!(target: "execution", particle_id = particle_id, "Executing particle");

            let particle_params = ParticleParameters {
                current_peer_id: Cow::Owned(current_peer_id.to_string()),
                init_peer_id: Cow::Owned(particle.init_peer_id.to_string()),
                // we use signature hex as particle id to prevent compromising of particle data store
                particle_id: Cow::Borrowed(&particle_id),
                timestamp: particle.timestamp,
                ttl: particle.ttl,
            };
            let prev_data_len = prev_data.len();
            let air = particle.script;
            let current_data = particle.data;
            let avm_outcome = self.call(
                air.clone(),
                current_data.clone(),
                prev_data.clone(),
                particle_params.clone(),
                call_results.clone(),
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

            match &avm_outcome {
                Ok(outcome) => {
                    let len = outcome.data.len();

                    tracing::trace!(
                        target: "execution", particle_id = particle_id,
                        "Particle interpreted in {} [{} bytes => {} bytes]",
                        humantime::format_duration(interpretation_time), prev_data_len, len
                    );

                    if data_store.detect_anomaly(interpretation_time, memory_delta, outcome) {
                        let anomaly_result = data_store
                            .save_anomaly_data(
                                air.as_str(),
                                &prev_data,
                                &current_data,
                                &call_results,
                                &particle_params,
                                outcome,
                                interpretation_time,
                                memory_delta,
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
                            runtime: Some(self),
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
                runtime: Some(self),
                effects,
                stats,
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
