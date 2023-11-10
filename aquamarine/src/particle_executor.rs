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
use std::time::Duration;
use std::{task::Waker, time::Instant};

use avm_server::{CallResults, ParticleParameters};
use fluence_keypair::KeyPair;
use futures::{future::BoxFuture, FutureExt};
use humantime::format_duration as pretty;

use fluence_libp2p::PeerId;
use particle_protocol::Particle;

use crate::aqua_runtime::AquaRuntime;
use crate::particle_effects::ParticleEffects;
use crate::InterpretationStats;

pub(super) type AVMRes<RT> = FutResult<RT, ParticleEffects, InterpretationStats>;
pub(super) type Fut<RT> = BoxFuture<'static, AVMRes<RT>>;

pub trait ParticleExecutor {
    type Future;
    type Particle;
    fn execute(
        self,
        p: Self::Particle,
        waker: Waker,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Future;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult<RT, Eff, Stats> {
    /// Return back AVM that just executed a particle and other reusable entities needed for execution
    pub runtime: RT,
    /// Effects produced by particle execution
    pub effects: Eff,
    /// Performance stats
    pub stats: Stats,
}

impl<RT: AquaRuntime> ParticleExecutor for RT {
    type Future = Fut<Option<Self>>;
    type Particle = (Particle, CallResults);

    fn execute(
        mut self,
        p: Self::Particle,
        waker: Waker,
        current_peer_id: PeerId,
        key_pair: KeyPair,
    ) -> Self::Future {
        let (particle, calls) = p;
        let particle_id = particle.id.clone();
        let data_len = particle.data.len();
        let span = tracing::info_span!("Execute");
        let task = tokio::task::Builder::new()
            .name(&format!("Particle {}", particle.id))
            .spawn_blocking(move || {
                span.in_scope(move || {
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
                    let result = self.call(
                        particle.script,
                        particle.data,
                        particle_params,
                        calls,
                        &key_pair,
                    );

                    let call_time = now.elapsed();
                    let new_data_len = result.as_ref().map(|e| e.data.len()).ok();
                    let mut stats = InterpretationStats {
                        interpretation_time: Duration::ZERO,  //TODO: here we don't have information for the interpretation time with the error that happened, fix after refactoring AVM::call to AVMRunner::call 
                        call_time,
                        new_data_len,
                        success: result.is_ok(),
                    };


                    match &result {
                        Ok(outcome) => {
                            stats.interpretation_time = outcome.execution_time;
                            let len = new_data_len.map(|l| l as i32).unwrap_or(-1);
                            tracing::trace!(
                            target: "execution", particle_id = particle.id,
                            "Particle interpreted in {} [{} bytes => {} bytes]",
                            pretty(call_time), data_len, len
                        );
                        },
                        Err(err) => {
                            tracing::warn!(
                            particle_id = particle.id,
                            "Error executing particle: {}",
                            err
                        )
                        }
                    }

                    let effects = Self::into_effects(result, particle.id);

                    waker.wake();

                    FutResult {
                        runtime: Some(self),
                        effects,
                        stats,
                    }
                })
            })
            .expect("Could not spawn 'Particle' task");

        async move {
            let result = task.await;
            match result {
                Ok(res) => res,
                Err(err) => {
                    if err.is_cancelled() {
                        tracing::warn!(particle_id, "Particle task was cancelled");
                    } else {
                        tracing::error!(particle_id, "Particle task panic");
                    }
                    let stats = InterpretationStats {
                        interpretation_time: Duration::ZERO,
                        call_time: Duration::ZERO,
                        new_data_len: None,
                        success: false,
                    };
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
        }
        .boxed()
    }
}
