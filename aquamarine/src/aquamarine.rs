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
use std::future::Future;
use std::task::Poll;
use std::time::Duration;

use async_std::{task, task::JoinHandle};
use futures::{channel::mpsc, SinkExt, StreamExt};

use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, Outlet};
use particle_execution::ParticleFunctionStatic;
use particle_protocol::Particle;

use crate::aqua_runtime::AquaRuntime;
use crate::command::Command;
use crate::command::Command::Ingest;
use crate::error::AquamarineApiError;
use crate::particle_effects::NetworkEffects;
use crate::particle_functions::Function;
use crate::vm_pool::VmPool;
use crate::{Plumber, VmPoolConfig};

pub type EffectsChannel = Outlet<Result<NetworkEffects, AquamarineApiError>>;

pub struct AquamarineBackend<RT: AquaRuntime, F> {
    inlet: BackPressuredInlet<Command>,
    plumber: Plumber<RT, F>,
    out: EffectsChannel,
}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> AquamarineBackend<RT, F> {
    pub fn new(
        config: VmPoolConfig,
        runtime_config: RT::Config,
        builtins: F,
        out: EffectsChannel,
    ) -> (Self, AquamarineApi) {
        let (outlet, inlet) = mpsc::channel(100);
        let sender = AquamarineApi::new(outlet, config.execution_timeout);
        let vm_pool = VmPool::new(config.pool_size, runtime_config);
        let plumber = Plumber::new(vm_pool, builtins);
        let this = Self {
            inlet,
            plumber,
            out,
        };

        (this, sender)
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let mut wake = false;

        // check if there are new particles
        while let Poll::Ready(Some(Ingest { particle, function })) = self.inlet.poll_next_unpin(cx)
        {
            wake = true;
            // set new particle to be executed
            self.plumber.ingest(particle, function);
        }

        // check if there are executed particles
        while let Poll::Ready(effects) = self.plumber.poll(cx) {
            wake = true;
            // send results back
            let sent = self.out.unbounded_send(effects);
            if let Err(err) = sent {
                log::error!("Aquamarine effects outlet has died: {}", err);
            }
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

#[derive(Clone)]
pub struct AquamarineApi {
    outlet: BackPressuredOutlet<Command>,
    #[allow(dead_code)]
    execution_timeout: Duration,
}

impl AquamarineApi {
    pub fn new(outlet: BackPressuredOutlet<Command>, execution_timeout: Duration) -> Self {
        Self {
            outlet,
            execution_timeout,
        }
    }

    /// Send particle to the interpreters pool
    pub fn execute(
        self,
        particle: Particle,
        function: Option<Function>,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        use AquamarineApiError::*;

        let mut interpreters = self.outlet;
        let particle_id = particle.id.clone();

        async move {
            let command = Ingest { particle, function };
            let sent = interpreters.send(command).await;

            sent.map_err(|err| match err {
                err if err.is_disconnected() => {
                    log::error!("Aquamarine outlet died!");
                    AquamarineDied { particle_id }
                }
                _ /* if err.is_full() */ => {
                    // This couldn't happen AFAIU, because `SinkExt::send` checks for availability
                    log::error!("UNREACHABLE: Aquamarine outlet reported being full!");
                    AquamarineQueueFull { particle_id }
                }
            })
        }
    }
}
