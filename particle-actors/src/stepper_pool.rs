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
use crate::{AwaitedEffects, AwaitedParticle, Plumber, StepperEffects, VmPoolConfig};
use async_std::task;
use async_std::task::JoinHandle;
use fluence_libp2p::types::{BackPressuredInlet, BackPressuredOutlet, OneshotOutlet};
use futures::channel::oneshot::Canceled;
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::{FutureExt, SinkExt, StreamExt};
use host_closure::ClosureDescriptor;
use particle_protocol::Particle;
use std::task::Poll;

pub struct StepperPoolProcessor {
    inlet: BackPressuredInlet<(Particle, OneshotOutlet<StepperEffects>)>,
    plumber: Plumber,
}

impl StepperPoolProcessor {
    pub fn new(
        config: VmPoolConfig,
        host_closures: ClosureDescriptor,
    ) -> (Self, StepperPoolSender) {
        let (outlet, inlet) = mpsc::channel(100);
        let plumber = Plumber::new(config, host_closures);
        let this = Self { inlet, plumber };
        let sender = StepperPoolSender::new(outlet);

        (this, sender)
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if let Poll::Ready(Some((particle, out))) = self.inlet.poll_next_unpin(cx) {
            self.plumber.ingest(AwaitedParticle { particle, out });
        }

        if let Poll::Ready(AwaitedEffects { effects, out }) = self.plumber.poll(cx) {
            out.send(effects).ok();
        }

        Poll::Pending
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
pub struct StepperPoolSender {
    // send particle along with a "return address"; it's like the Ask pattern in Akka
    outlet: BackPressuredOutlet<(Particle, OneshotOutlet<StepperEffects>)>,
}
impl StepperPoolSender {
    pub fn new(outlet: BackPressuredOutlet<(Particle, OneshotOutlet<StepperEffects>)>) -> Self {
        Self { outlet }
    }

    /// Send particle to interpreters pool and wait response back
    pub fn ingest(
        self,
        particle: Particle,
    ) -> BoxFuture<'static, Result<StepperEffects, Canceled>> {
        let mut interpreters = self.outlet;
        async move {
            let (outlet, inlet) = oneshot::channel();
            interpreters
                .send((particle, outlet))
                .await
                .expect("interpreter pool died?");
            inlet.await
        }
        .boxed()
    }
}
