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

use std::{
    collections::VecDeque,
    task::{Context, Poll, Waker},
};

use avm_server::CallResults;
use futures::FutureExt;

use particle_execution::ParticleFunction;

use crate::awaited_particle::AwaitedParticle;
use crate::deadline::Deadline;
use crate::error::AquamarineApiError;
use crate::particle_effects::NetworkEffects;
use crate::particle_executor::{Fut, FutResult, ParticleExecutor};
use crate::particle_functions::Functions;
use crate::AwaitedEffects;

pub struct Actor<RT, F> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<Fut<RT>>,
    mailbox: VecDeque<AwaitedParticle>,
    waker: Option<Waker>,
    functions: Functions<F>,
}

impl<RT, F> Actor<RT, F>
where
    RT: ParticleExecutor<Particle = (AwaitedParticle, CallResults), Future = Fut<RT>>,
    F: ParticleFunction + 'static,
{
    pub fn new(deadline: Deadline, functions: Functions<F>) -> Self {
        Self {
            deadline,
            functions,
            future: None,
            mailbox: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.deadline.is_expired(now_ms)
    }

    pub fn is_executing(&self) -> bool {
        self.future.is_some()
    }

    pub fn cleanup(&self, particle_id: &str) -> eyre::Result<()> {
        // TODO: remove vault and particle data, maybe also particle_functions?
        todo!("{}", particle_id)
    }

    pub fn mailbox_size(&self) -> usize {
        self.mailbox.len()
    }

    pub fn ingest(&mut self, particle: AwaitedParticle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(&mut self, cx: &mut Context<'_>) -> Poll<FutResult<RT, NetworkEffects>> {
        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Poll AquaVM future
        if let Some(Poll::Ready(r)) = self.future.as_mut().map(|f| f.poll_unpin(cx)) {
            self.future.take();

            let effects = match r.effects.effects {
                Ok(effects) => {
                    // Schedule execution of functions
                    self.functions.execute(effects.call_requests);
                    Ok(NetworkEffects {
                        particle: effects.particle,
                        next_peers: effects.next_peers,
                    })
                }
                Err(err) => Err(err),
            };

            return Poll::Ready(FutResult {
                vm: r.vm,
                effects: AwaitedEffects {
                    effects,
                    out: r.effects.out,
                },
            });
        }

        Poll::Pending

        // Poll self.future
        // let future = self.future.take().map(|mut fut| (fut.poll_unpin(cx), fut));
        // match future {
        //     // If future is ready, return effects and vm
        //     Some((Poll::Ready(r), _)) => Poll::Ready(r),
        //     o => {
        //         // Either keep pending future or keep it None
        //         self.future = o.map(|(_, fut)| fut);
        //         Poll::Pending
        //     }
        // }
    }

    /// Provide actor with new `vm` to execute particles, if there are any.
    ///
    /// If actor is in the middle of executing previous particle, vm is returned
    /// If actor's mailbox is empty, vm is returned
    pub fn poll_next(&mut self, vm: RT, cx: &mut Context<'_>) -> ActorPoll<RT> {
        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Return vm if previous particle is still executing
        if self.is_executing() {
            return ActorPoll::Vm(vm);
        }

        match self.mailbox.pop_front() {
            Some(p) if !p.is_expired() => {
                // Gather CallResults
                let calls = self.functions.drain();

                // TODO: add timeout for execution
                // Take ownership of vm to process particle
                self.future = Some(vm.execute((p, calls), cx.waker().clone()));
                ActorPoll::Executing
            }
            Some(p) => {
                // Particle is expired, return vm and error
                let (p, out) = p.into();
                let particle_id = p.id;
                let effects = Err(AquamarineApiError::ParticleExpired { particle_id });
                let effects = AwaitedEffects { effects, out };
                ActorPoll::Expired(effects, vm)
            }
            // Mailbox is empty, return vm
            None => ActorPoll::Vm(vm),
        }
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

pub enum ActorPoll<RT> {
    Executing,
    Vm(RT),
    Expired(AwaitedEffects<NetworkEffects>, RT),
}
