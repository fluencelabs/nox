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
use std::convert::identity;
use std::task::Poll;
use std::time::Duration;

use async_std::{task, task::JoinHandle};
use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
    FutureExt, SinkExt, StreamExt,
};
use humantime::format_duration as pretty;

use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet};

use crate::aqua_runtime::AquaRuntime;
use crate::awaited_particle::EffectsChannel;
use crate::error::AquamarineApiError;
use crate::observation::Observation;
use crate::{AwaitedEffects, AwaitedParticle, ParticleEffects, Plumber, VmPoolConfig};

pub struct AquamarineBackend<RT: AquaRuntime> {
    inlet: BackPressuredInlet<(Observation, EffectsChannel)>,
    plumber: Plumber<RT>,
}

impl<RT: AquaRuntime> AquamarineBackend<RT> {
    pub fn new(config: VmPoolConfig, runtime_config: RT::Config) -> (Self, AquamarineApi) {
        let (outlet, inlet) = mpsc::channel(100);
        let sender = AquamarineApi::new(outlet, config.execution_timeout);
        let plumber = Plumber::new(config, runtime_config);
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
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let mut stream = futures::stream::poll_fn(move |cx| self.poll(cx).map(|_| Some(()))).fuse();
        task::spawn(async move {
            loop {
                stream.next().await;
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct AquamarineApi {
    // send particle along with a "return address"; it's like the Ask pattern in Akka
    outlet: BackPressuredOutlet<(Observation, EffectsChannel)>,
    execution_timeout: Duration,
}
impl AquamarineApi {
    pub fn new(
        outlet: BackPressuredOutlet<(Observation, EffectsChannel)>,
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
        particle: Observation,
    ) -> BoxFuture<'static, Result<ParticleEffects, AquamarineApiError>> {
        use AquamarineApiError::*;

        let mut interpreters = self.outlet;
        let particle_id = particle.id.clone();
        let fut = async move {
            let particle_id = particle.id.clone();
            let (outlet, inlet) = oneshot::channel();
            let send_ok = interpreters.send((particle, outlet)).await.is_ok();
            if send_ok {
                let effects = inlet.await.map_err(|err| {
                    log::info!(target: "debug", "oneshot cancelled: {:?}", err);
                    OneshotCancelled { particle_id }
                });
                effects.and_then(identity)
            } else {
                Err(AquamarineDied { particle_id })
            }
        };

        let timeout = self.execution_timeout;
        async_std::io::timeout(timeout, fut.map(Ok))
            .map(move |r| {
                let result = r.map_err(|_| ExecutionTimedOut {
                    particle_id,
                    timeout: pretty(timeout),
                });
                result.and_then(identity)
            })
            .boxed()
    }
}
