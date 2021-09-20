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

use crate::aqua_runtime::AquaRuntime;
use crate::awaited_particle::AwaitedParticle;
use crate::AwaitedEffects;

use avm_server::CallResults;
use particle_protocol::Particle;

use async_std::task;
use futures::{future::BoxFuture, FutureExt};
use humantime::format_duration as pretty;
use std::{task::Waker, time::Instant};

pub(super) type Fut<RT> = BoxFuture<'static, FutResult<RT>>;

pub trait ParticleExecutor {
    type Future;
    type Particle;
    fn execute(self, p: Self::Particle, waker: Waker) -> Self::Future;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult<RT> {
    /// AVM that just executed a particle
    pub vm: RT,
    /// Effects produced by particle execution
    pub effects: AwaitedEffects,
}

impl<RT: AquaRuntime> ParticleExecutor for RT {
    type Future = Fut<Self>;
    type Particle = AwaitedParticle;

    fn execute(mut self, p: AwaitedParticle, waker: Waker) -> Self::Future {
        task::spawn_blocking(move || {
            let now = Instant::now();
            log::info!("Executing particle {}", p.id);

            let (observation, out) = p.into();
            let (p, calls): (Particle, CallResults) = observation.into();

            let result = self.call(p.init_peer_id, p.script.clone(), p.data.clone(), &p.id, &calls);
            if let Err(err) = &result {
                log::warn!("Error executing particle {:#?}: {}", p, err)
            } else {
                log::trace!(target: "network", "Particle {} executed in {}", p.id, pretty(now.elapsed()));
            }
            let effects = Ok(Self::into_effects(result, p));

            waker.wake();

            FutResult {
                vm: self,
                effects: AwaitedEffects { effects, out },
            }
        })
        .boxed()
    }
}
