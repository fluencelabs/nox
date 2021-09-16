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

use crate::config::VmConfig;
use crate::invoke::{parse_outcome, ExecutionError};
use crate::particle_data_store::ParticleDataStore;
use crate::{ParticleEffects, SendParticle};
use avm_server::{AVMConfig, AVMDataStore, AVMError, AVMOutcome, CallResults, AVM};
use host_closure::ClosureDescriptor;
use particle_protocol::Particle;

use async_std::task;
use futures::{future::BoxFuture, FutureExt};
use libp2p::PeerId;
use log::LevelFilter;
use std::{error::Error, task::Waker};

pub trait AquaRuntime: Sized + Send + 'static {
    type Config: Clone + Send + 'static;
    type Error: Error;

    fn create_runtime(config: Self::Config, waker: Waker) -> BoxFuture<'static, Result<Self, Self::Error>>;

    // TODO: move into_effects inside call
    fn into_effects(outcome: Result<AVMOutcome, Self::Error>, p: Particle) -> ParticleEffects;

    fn call(
        &mut self,
        init_user_id: PeerId,
        aqua: String,
        data: Vec<u8>,
        particle_id: String,
        call_results: &CallResults,
    ) -> Result<AVMOutcome, Self::Error>;

    fn cleanup(&self, particle_id: &str) -> Result<(), Self::Error>;
}

impl AquaRuntime for AVM {
    type Config = VmConfig;
    type Error = AVMError;

    /// Creates `AVM` in background (on blocking threadpool)
    fn create_runtime(config: Self::Config, waker: Waker) -> BoxFuture<'static, Result<Self, Self::Error>> {
        task::spawn_blocking(move || {
            let data_store = Box::new(ParticleDataStore {
                particle_data_store: config.particles_dir,
                vault_dir: config.particles_vault_dir,
            });
            let config = AVMConfig {
                data_store,
                current_peer_id: config.current_peer_id.to_string(),
                air_wasm_path: config.air_interpreter,
                logging_mask: i32::MAX,
            };
            let vm = AVM::new(config);
            waker.wake();
            vm
        })
        .boxed()
    }

    fn into_effects(outcome: Result<AVMOutcome, AVMError>, p: Particle) -> ParticleEffects {
        match parse_outcome(outcome) {
            Ok((data, peers, calls)) if !peers.is_empty() || !calls.is_empty() => {
                #[rustfmt::skip]
                log::debug!("Particle {} executed, will be sent to {} targets", p.id, targets.len());

                ParticleEffects {
                    next_peers,
                    call_requests: calls,
                    particle: Particle { data, ..p },
                }
            }
            Ok((data, _)) => {
                log::warn!(
                    "Executed particle {}, next_peer_pks is empty, no call requests. Nothing to do.",
                    p.id
                );
                if log::max_level() >= LevelFilter::Debug {
                    let data = String::from_utf8_lossy(data.as_slice());
                    log::debug!("particle {} next_peer_pks = [], data: {}", p.id, data);
                }
                <_>::default()
            }
            Err(ExecutionError::AquamarineError(err)) => {
                log::warn!("Error executing particle {:#?}: {}", p, err);
                <_>::default()
            }
            Err(err @ ExecutionError::AVMOutcome { .. }) => {
                log::warn!("Error executing script: {}", err);
                <_>::default()
            }
            Err(err @ ExecutionError::InvalidResultField { .. }) => {
                log::warn!("Error parsing outcome for particle {:#?}: {}", p, err);
                <_>::default()
            }
        }
    }

    #[inline]
    fn call(
        &mut self,
        init_user_id: PeerId,
        aqua: String,
        data: Vec<u8>,
        particle_id: &str,
        call_results: &CallResults,
    ) -> Result<AVMOutcome, Self::Error> {
        AVM::call(self, init_user_id.to_string(), aqua, data, particle_id, call_results)
    }

    #[inline]
    fn cleanup(&self, particle_id: &str) -> Result<(), Self::Error> {
        AVM::cleanup_particle(self, particle_id)
    }
}
