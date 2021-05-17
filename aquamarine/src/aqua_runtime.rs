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
use crate::{SendParticle, StepperEffects};

use avm_server::{AVMConfig, AVMError, InterpreterOutcome, AVM};
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

    fn create_runtime(
        config: Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>>;

    // TODO: move into_effects inside call
    fn into_effects(
        outcome: Result<InterpreterOutcome, Self::Error>,
        p: Particle,
    ) -> StepperEffects;

    fn call(
        &mut self,
        init_user_id: PeerId,
        aqua: String,
        data: Vec<u8>,
        particle_id: String,
        // TODO: return StepperEffects
    ) -> Result<InterpreterOutcome, Self::Error>;
}

impl AquaRuntime for AVM {
    type Config = (VmConfig, ClosureDescriptor);
    type Error = AVMError;

    /// Creates `AVM` in background (on blocking threadpool)
    fn create_runtime(
        (config, host_closure): Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>> {
        task::spawn_blocking(move || {
            let config = AVMConfig {
                current_peer_id: config.current_peer_id.to_string(),
                air_wasm_path: config.air_interpreter,
                particle_data_store: config.particles_dir,
                call_service: host_closure(),
                logging_mask: i32::MAX,
            };
            let vm = AVM::new(config);
            waker.wake();
            vm
        })
        .boxed()
    }

    fn into_effects(outcome: Result<InterpreterOutcome, AVMError>, p: Particle) -> StepperEffects {
        let particles = match parse_outcome(outcome) {
            Ok((data, targets)) if !targets.is_empty() => {
                #[rustfmt::skip]
                log::debug!("Particle {} executed, will be sent to {} targets", p.id, targets.len());
                let particle = Particle { data, ..p };
                targets
                    .into_iter()
                    .map(|target| SendParticle {
                        particle: particle.clone(),
                        target,
                    })
                    .collect::<Vec<_>>()
            }
            Ok((data, _)) => {
                log::warn!(
                    "Executed particle {}, next_peer_pks is empty. Won't send anywhere",
                    p.id
                );
                if log::max_level() >= LevelFilter::Debug {
                    let data = String::from_utf8_lossy(data.as_slice());
                    log::debug!("particle {} next_peer_pks = [], data: {}", p.id, data);
                }
                vec![]
            }
            Err(ExecutionError::AquamarineError(err)) => {
                log::warn!("Error executing particle {:#?}: {}", p, err);
                vec![]
            }
            Err(err @ ExecutionError::InterpreterOutcome { .. }) => {
                log::warn!("Error executing script: {}", err);
                vec![]
            }
            Err(err @ ExecutionError::InvalidResultField { .. }) => {
                log::warn!("Error parsing outcome for particle {:#?}: {}", p, err);
                vec![]
            }
        };

        StepperEffects { particles }
    }

    #[inline]
    fn call(
        &mut self,
        init_user_id: PeerId,
        aqua: String,
        data: Vec<u8>,
        particle_id: String,
    ) -> Result<InterpreterOutcome, Self::Error> {
        AVM::call(self, init_user_id.to_string(), aqua, data, particle_id)
    }
}
