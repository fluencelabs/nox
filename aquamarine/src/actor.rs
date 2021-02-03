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
use crate::particle_executor::{Fut, FutResult, ParticleExecutor};

use aquamarine_vm::AquamarineVM;
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

pub struct Actor {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<Fut>,
    mailbox: VecDeque<AwaitedParticle>,
    waker: Option<Waker>,
}

impl Actor {
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

    pub fn ingest(&mut self, particle: AwaitedParticle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(&mut self, cx: &mut Context<'_>) -> Poll<FutResult> {
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
    pub fn poll_next(&mut self, vm: AquamarineVM, cx: &mut Context<'_>) -> Poll<AquamarineVM> {
        self.waker = Some(cx.waker().clone());

        // Return vm if previous particle is still executing
        if self.future.is_some() {
            return Poll::Ready(vm);
        }

        match self.mailbox.pop_front() {
            Some(p) => {
                // Take ownership of vm to process particle
                self.future = vm.execute(p, cx.waker().clone()).into();
                Poll::Pending
            }
            // Mailbox is empty, return vm
            None => Poll::Ready(vm),
        }
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}
