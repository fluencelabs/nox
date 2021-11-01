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

use std::{task::Waker, time::Instant};

use async_std::task;
use avm_server::CallResults;
use futures::{future::BoxFuture, FutureExt};
use humantime::format_duration as pretty;

use particle_protocol::Particle;

use crate::aqua_runtime::AquaRuntime;
use crate::error::AquamarineApiError;
use crate::particle_effects::ParticleEffects;

pub(super) type Fut<RT> = BoxFuture<'static, FutResult<RT, ParticleEffects>>;

pub trait ParticleExecutor {
    type Future;
    type Particle;
    fn execute(self, p: Self::Particle, waker: Waker) -> Self::Future;
}

/// Result of a particle execution along a VM that has just executed the particle
pub struct FutResult<RT, Eff> {
    /// AVM that just executed a particle
    pub vm: RT,
    /// Effects produced by particle execution
    pub effects: Result<Eff, AquamarineApiError>,
}

impl<RT: AquaRuntime> ParticleExecutor for RT {
    type Future = Fut<Self>;
    type Particle = (Particle, CallResults);

    fn execute(mut self, p: Self::Particle, waker: Waker) -> Self::Future {
        task::spawn_blocking(move || {
            let now = Instant::now();
            let (p, calls) = p;
            log::info!("Executing particle {}", p.id);

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
                effects,
            }
        })
        .boxed()
    }
}
