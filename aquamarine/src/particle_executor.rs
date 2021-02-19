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

use crate::awaited_particle::AwaitedParticle;
use crate::invoke::{parse_outcome, ExecutionError};
use crate::{AwaitedEffects, SendParticle, StepperEffects};
use aquamarine_vm::{AquamarineVM, AquamarineVMError, StepperOutcome};
use async_std::task;
use futures::future::BoxFuture;
use futures::FutureExt;
use log::LevelFilter;
use particle_protocol::Particle;
use std::task::Waker;

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub trait ParticleExecutor {
    type Future;
    type Particle;
    fn execute(self, p: Self::Particle, waker: Waker) -> Self::Future;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult {
    /// AquamarineVM that just executed a particle
    pub vm: AquamarineVM,
    /// Effects produced by particle execution
    pub effects: AwaitedEffects,
}

impl ParticleExecutor for AquamarineVM {
    type Future = Fut;
    type Particle = AwaitedParticle;

    fn execute(mut self, p: AwaitedParticle, waker: Waker) -> Fut {
        task::spawn_blocking(move || {
            log::info!("Executing particle {}", p.id);

            let (p, out) = p.into();

            let init_peer_id = p.init_peer_id.to_string();
            let result = self.call(init_peer_id, &p.script, p.data.clone(), &p.id);
            if let Err(err) = &result {
                log::warn!("Error executing particle {:#?}: {}", p, err)
            }
            let effects = Ok(into_effects(result, p));

            waker.wake();

            FutResult {
                vm: self,
                effects: AwaitedEffects { out, effects },
            }
        })
        .boxed()
    }
}

fn into_effects(outcome: Result<StepperOutcome, AquamarineVMError>, p: Particle) -> StepperEffects {
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
        Err(err @ ExecutionError::StepperOutcome { .. }) => {
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
