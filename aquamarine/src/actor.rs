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

use fluence_libp2p::PeerId;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::Particle;

use crate::deadline::Deadline;
use crate::particle_effects::NetworkEffects;
use crate::particle_executor::{Fut, FutResult, ParticleExecutor};
use crate::particle_functions::{Functions, SingleCallStat};
use crate::{AquaRuntime, InterpretationStats};

pub struct Actor<RT, F> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<(usize, Fut<RT>)>,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
    functions: Functions<F>,
    /// Particle that's memoized on the actor creation.
    /// Used to execute CallRequests when mailbox is empty.
    /// Particle's data is empty.
    particle: Particle,
    /// Particles and call results will be processed in the security scope of this peer id
    /// It's either `host_peer_id` or owner-specific spell peer id
    current_peer_id: PeerId,
}

impl<RT, F> Actor<RT, F>
where
    RT: AquaRuntime + ParticleExecutor<Particle = (Particle, CallResults), Future = Fut<RT>>,
    F: ParticleFunctionStatic,
{
    pub fn new(particle: &Particle, functions: Functions<F>, current_peer_id: PeerId) -> Self {
        Self {
            deadline: Deadline::from(&particle),
            functions,
            future: None,
            mailbox: <_>::default(),
            waker: None,
            // Clone particle without data
            particle: Particle {
                id: particle.id.clone(),
                init_peer_id: particle.init_peer_id,
                timestamp: particle.timestamp,
                ttl: particle.ttl,
                script: particle.script.clone(),
                signature: particle.signature.clone(),
                data: vec![],
            },
            current_peer_id,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.deadline.is_expired(now_ms)
    }

    pub fn is_executing(&self) -> bool {
        self.future.is_some()
    }

    pub fn cleanup(&self, particle_id: &str, vm: &mut RT) -> eyre::Result<()> {
        // TODO: remove dirs without using vm https://github.com/fluencelabs/fluence/issues/1216
        vm.cleanup(particle_id)?;
        Ok(())
    }

    pub fn mailbox_size(&self) -> usize {
        self.mailbox.len()
    }

    pub fn set_function(&mut self, function: ServiceFunction) {
        self.functions.set_function(function)
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<FutResult<(usize, RT), NetworkEffects, InterpretationStats>> {
        use Poll::Ready;

        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Poll AquaVM future
        if let Some((vm_id, Ready(r))) = self.future.as_mut().map(|(i, f)| (*i, f.poll_unpin(cx))) {
            self.future.take();

            let waker = cx.waker().clone();
            // Schedule execution of functions
            self.functions.execute(r.effects.call_requests, waker);

            let effects = NetworkEffects {
                particle: r.effects.particle,
                next_peers: r.effects.next_peers,
            };
            return Poll::Ready(FutResult {
                vm: (vm_id, r.vm),
                effects,
                stats: r.stats,
            });
        }

        Poll::Pending
    }

    /// Provide actor with new `vm` to execute particles, if there are any.
    ///
    /// If actor is in the middle of executing previous particle, vm is returned
    /// If actor's mailbox is empty, vm is returned
    pub fn poll_next(&mut self, vm_id: usize, vm: RT, cx: &mut Context<'_>) -> ActorPoll<RT> {
        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Return vm if previous particle is still executing
        if self.is_executing() {
            return ActorPoll::Vm(vm_id, vm);
        }

        // Gather CallResults
        let (calls, stats) = self.functions.drain();

        // Take the next particle
        let particle = self.mailbox.pop_front();

        if particle.is_none() && calls.is_empty() {
            debug_assert!(stats.is_empty(), "stats must be empty if calls are empty");
            // Nothing to execute, return vm
            return ActorPoll::Vm(vm_id, vm);
        }

        let particle = particle.unwrap_or_else(|| {
            // If mailbox is empty, then take self.particle.
            // Its data is empty, so `vm` will process `calls` on the old (saved on disk) data
            self.particle.clone()
        });
        let waker = cx.waker().clone();
        // TODO: add timeout for execution https://github.com/fluencelabs/fluence/issues/1212
        // Take ownership of vm to process particle
        self.future = Some((
            vm_id,
            vm.execute((particle, calls), waker, self.current_peer_id.clone()),
        ));

        ActorPoll::Executing(stats)
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

pub enum ActorPoll<RT> {
    Executing(Vec<SingleCallStat>),
    Vm(usize, RT),
}
