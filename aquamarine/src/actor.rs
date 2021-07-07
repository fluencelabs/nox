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
use crate::error::AquamarineApiError;
use crate::particle_executor::{Fut, FutResult, ParticleExecutor};
use crate::AwaitedEffects;

use particle_protocol::Particle;

use futures::FutureExt;
use std::ops::Mul;
use std::{
    collections::VecDeque,
    fmt::Debug,
    task::{Context, Poll, Waker},
};

#[derive(Debug, Clone)]
pub struct Deadline {
    timestamp: u64,
    ttl: u32,
}

impl Deadline {
    pub fn from(particle: impl AsRef<Particle>) -> Self {
        let particle = particle.as_ref();
        Self {
            timestamp: particle.timestamp,
            ttl: particle.ttl,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.timestamp
            .mul(1000)
            .checked_add(self.ttl as u64)
            // Whether ts is in the past
            .map(|ts| ts < now_ms)
            // If timestamp + ttl gives overflow, consider particle expired
            .unwrap_or_else(|| {
                log::warn!("timestamp {} + ttl {} overflowed", self.timestamp, self.ttl);
                true
            })
    }
}

pub struct Actor<RT> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<Fut<RT>>,
    mailbox: VecDeque<AwaitedParticle>,
    waker: Option<Waker>,
}

impl<RT> Actor<RT>
where
    RT: ParticleExecutor<Particle = AwaitedParticle, Future = Fut<RT>>,
{
    pub fn new(deadline: Deadline) -> Self {
        Self {
            deadline,
            future: None,
            mailbox: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.deadline.is_expired(now_ms)
    }

    pub fn mailbox_size(&self) -> usize {
        self.mailbox.len()
    }

    pub fn ingest(&mut self, particle: AwaitedParticle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(&mut self, cx: &mut Context<'_>) -> Poll<FutResult<RT>> {
        self.waker = Some(cx.waker().clone());

        // Poll self.future
        let future = self.future.take().map(|mut fut| (fut.poll_unpin(cx), fut));

        match future {
            // If future is ready, return effects and vm
            Some((Poll::Ready(r), _)) => Poll::Ready(r),
            o => {
                // Either keep pending future or keep it None
                self.future = o.map(|t| t.1);
                Poll::Pending
            }
        }
    }

    /// Provide actor with new `vm` to execute particles, if there are any.
    ///
    /// If actor is in the middle of executing previous particle, vm is returned
    /// If actor's mailbox is empty, vm is returned
    pub fn poll_next(&mut self, vm: RT, cx: &mut Context<'_>) -> ActorPoll<RT> {
        self.waker = Some(cx.waker().clone());

        // Return vm if previous particle is still executing
        if self.future.is_some() {
            return ActorPoll::Vm(vm);
        }

        match self.mailbox.pop_front() {
            Some(p) if !p.is_expired() => {
                // Take ownership of vm to process particle
                // TODO: add timeout for execution
                self.future = vm.execute(p, cx.waker().clone()).into();
                ActorPoll::Executing
            }
            Some(p) => {
                // Particle is expired, return vm and error
                let (p, out) = p.into();
                let effects = Err(AquamarineApiError::ParticleExpired { particle_id: p.id });
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
    Expired(AwaitedEffects, RT),
}
