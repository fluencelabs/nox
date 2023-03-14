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

use std::{error::Error, task::Waker};

use avm_server::{
    AVMConfig, AVMError, AVMMemoryStats, AVMOutcome, CallResults, ParticleParameters, AVM,
};
use futures::{future::BoxFuture, FutureExt};
use log::LevelFilter;

use particle_protocol::Particle;

use crate::config::VmConfig;
use crate::invoke::{parse_outcome, ExecutionError};
use crate::particle_data_store::{DataStoreError, ParticleDataStore};
use crate::particle_effects::ParticleEffects;

pub trait AquaRuntime: Sized + Send + 'static {
    type Config: Clone + Send + 'static;
    type Error: Error + Send + Sync + 'static;

    fn create_runtime(
        config: Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>>;

    // TODO: move into_effects inside call
    fn into_effects(outcome: Result<AVMOutcome, Self::Error>, p: Particle) -> ParticleEffects;

    fn call(
        &mut self,
        aqua: String,
        data: Vec<u8>,
        particle: ParticleParameters<'_>,
        call_results: CallResults,
    ) -> Result<AVMOutcome, Self::Error>;

    fn cleanup(&mut self, particle_id: &str, current_peer_id: &str) -> Result<(), Self::Error>;

    /// Return current size of memory. Use only for diagnostics purposes.
    fn memory_stats(&self) -> AVMMemoryStats;
}

#[derive(Debug, thiserror::Error)]
pub enum CreateAVMError {
    #[error(transparent)]
    AVWError(AVMError<DataStoreError>),
    #[error("Could not create AVM")]
    Fatal,
}

impl AquaRuntime for AVM<DataStoreError> {
    type Config = VmConfig;
    type Error = CreateAVMError;

    /// Creates `AVM` in background (on blocking threadpool)
    fn create_runtime(
        config: Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>> {
        let task = tokio::task::Builder::new()
            .name("Create AVM")
            .spawn_blocking(move || {
                let data_store = Box::new(ParticleDataStore::new(
                    config.particles_dir,
                    config.particles_vault_dir,
                    config.particles_anomaly_dir,
                ));
                let config = AVMConfig {
                    data_store,
                    air_wasm_path: config.air_interpreter,
                    logging_mask: i32::MAX,
                    max_heap_size: config.max_heap_size,
                };
                let vm = AVM::new(config)?;
                waker.wake();
                Ok(vm)
            })
            .expect("Could not spawn 'Create AVM' task");

        async {
            let result = task.await;
            match result {
                Ok(res) => res.map_err(CreateAVMError::AVWError),
                Err(e) => {
                    if e.is_cancelled() {
                        log::warn!("AVM creation task was cancelled");
                    } else {
                        log::error!("AVM creation task panic");
                    }
                    Err(CreateAVMError::Fatal)
                }
            }
        }
        .boxed()
    }

    fn into_effects(outcome: Result<AVMOutcome, CreateAVMError>, p: Particle) -> ParticleEffects {
        match parse_outcome(outcome) {
            Ok((data, peers, calls)) if !peers.is_empty() || !calls.is_empty() => {
                #[rustfmt::skip]
                log::debug!("Particle {} executed: {} call requests, {} next peers", p.id, calls.len(), peers.len());

                ParticleEffects {
                    next_peers: peers,
                    call_requests: calls,
                    particle: Particle { data, ..p },
                }
            }
            Ok((data, ..)) => {
                log::warn!(
                    "Executed particle {}, next_peer_pks is empty, no call requests. Nothing to do.",
                    p.id
                );
                if log::max_level() >= LevelFilter::Debug {
                    let data = String::from_utf8_lossy(data.as_slice());
                    log::debug!("particle {} next_peer_pks = [], data: {}", p.id, data);
                }
                ParticleEffects::empty(Particle { data, ..p })
            }
            Err(ExecutionError::AquamarineError(err)) => {
                log::warn!("Error executing particle {:#?}: {}", p, err);
                ParticleEffects::empty(p)
            }
            Err(err @ ExecutionError::InvalidResultField { .. }) => {
                log::warn!("Error parsing outcome for particle {:#?}: {}", p, err);
                ParticleEffects::empty(p)
            }
        }
    }

    #[inline]
    fn call(
        &mut self,
        aqua: String,
        data: Vec<u8>,
        particle: ParticleParameters<'_>,
        call_results: CallResults,
    ) -> Result<AVMOutcome, Self::Error> {
        AVM::call(self, aqua, data, particle, call_results)
            .map_err(CreateAVMError::AVWError)
    }

    #[inline]
    fn cleanup(&mut self, particle_id: &str, current_peer_id: &str) -> Result<(), Self::Error> {
        AVM::cleanup_data(self, particle_id, current_peer_id)
            .map_err(CreateAVMError::AVWError)
    }

    fn memory_stats(&self) -> AVMMemoryStats {
        self.memory_stats()
    }
}
