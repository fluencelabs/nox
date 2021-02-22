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
use crate::awaited_particle::EffectsChannel;
use crate::error::AquamarineApiError;
use crate::{AwaitedEffects, AwaitedParticle, Plumber, StepperEffects, VmPoolConfig};

use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet};
use host_closure::ClosureDescriptor;
use particle_protocol::Particle;

use async_std::{task, task::JoinHandle};
use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
    FutureExt, SinkExt, StreamExt,
};
use std::convert::identity;
use std::task::Poll;
use std::time::Duration;

pub struct AquamarineBackend {
    inlet: BackPressuredInlet<(Particle, EffectsChannel)>,
    plumber: Plumber,
}

impl AquamarineBackend {
    pub fn new(config: VmPoolConfig, host_closures: ClosureDescriptor) -> (Self, AquamarineApi) {
        let (outlet, inlet) = mpsc::channel(100);
        let sender = AquamarineApi::new(outlet, config.execution_timeout);
        let plumber = Plumber::new(config, host_closures);
        let this = Self { inlet, plumber };

        (this, sender)
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let mut wake = false;

        // check if there are new particles
        while let Poll::Ready(Some((particle, out))) = self.inlet.poll_next_unpin(cx) {
            wake = true;
            // set new particles to be executed
            self.plumber.ingest(AwaitedParticle { particle, out });
        }

        // check if there are executed particles
        while let Poll::Ready(AwaitedEffects { effects, out }) = self.plumber.poll(cx) {
            wake = true;
            // send results back
            out.send(effects).ok();
        }

        if wake {
            cx.waker().wake_by_ref();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let mut future = futures::future::poll_fn(move |cx| self.poll(cx)).into_stream();
        task::spawn(async move {
            loop {
                future.next().await;
            }
        })
    }
}

#[derive(Clone)]
pub struct AquamarineApi {
    // send particle along with a "return address"; it's like the Ask pattern in Akka
    outlet: BackPressuredOutlet<(Particle, EffectsChannel)>,
    execution_timeout: Duration,
}
impl AquamarineApi {
    pub fn new(
        outlet: BackPressuredOutlet<(Particle, EffectsChannel)>,
        execution_timeout: Duration,
    ) -> Self {
        Self {
            outlet,
            execution_timeout,
        }
    }

    /// Send particle to interpreters pool and wait response back
    pub fn handle(
        self,
        particle: Particle,
    ) -> BoxFuture<'static, Result<StepperEffects, AquamarineApiError>> {
        use AquamarineApiError::*;

        let mut interpreters = self.outlet;
        let particle_id = particle.id.clone();
        let fut = async move {
            let particle_id = particle.id.clone();
            let (outlet, inlet) = oneshot::channel();
            let send_ok = interpreters.send((particle, outlet)).await.is_ok();
            if send_ok {
                let effects = inlet.await.map_err(|_| OneshotCancelled { particle_id });
                effects.and_then(identity)
            } else {
                Err(AquamarineDied { particle_id })
            }
        };

        async_std::io::timeout(self.execution_timeout, fut.map(Ok))
            .map(|r| {
                let result = r.map_err(|_| ExecutionTimedOut { particle_id });
                result.and_then(identity)
            })
            .boxed()
    }
}
