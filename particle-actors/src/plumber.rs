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

use crate::actor::{Actor, ActorEvent};
use crate::config::VmPoolConfig;

use host_closure::ClosureDescriptor;
use particle_protocol::Particle;

use crate::vm_pool::VmPool;
use libp2p::PeerId;
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    fmt::Debug,
    task::{Context, Poll},
};

#[derive(Debug)]
pub enum PlumberEvent {
    Forward { target: PeerId, particle: Particle },
}

impl From<ActorEvent> for PlumberEvent {
    fn from(event: ActorEvent) -> Self {
        match event {
            ActorEvent::Forward { particle, target } => PlumberEvent::Forward { target, particle },
        }
    }
}

pub struct Plumber {
    events: VecDeque<PlumberEvent>,
    actors: HashMap<String, Actor>,
    vm_pool: VmPool,
}

impl Plumber {
    pub fn new(config: VmPoolConfig, host_closure: ClosureDescriptor) -> Self {
        let vm_pool = VmPool::new(config, host_closure);
        Self {
            vm_pool,
            events: <_>::default(),
            actors: <_>::default(),
        }
    }

    /// Receives and ingests incoming particle: creates a new actor or forwards to the existing mailbox
    pub fn ingest(&mut self, particle: Particle) {
        match self.actors.entry(particle.id.clone()) {
            Entry::Vacant(entry) => entry.insert(Actor::new(particle.clone())).ingest(particle),
            Entry::Occupied(mut entry) => entry.get_mut().ingest(particle),
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<PlumberEvent> {
        self.vm_pool.poll(cx);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // Gather effects and vms
        let mut effects = vec![];
        for actor in self.actors.values_mut() {
            if let Poll::Ready(result) = actor.poll_completed(cx) {
                effects.extend(result.effects);
                self.vm_pool.put_vm(result.vm);
            }
        }

        // Execute next messages
        for actor in self.actors.values_mut() {
            if let Some(vm) = self.vm_pool.get_vm() {
                if let Poll::Ready(vm) = actor.poll_next(vm, cx) {
                    self.vm_pool.put_vm(vm)
                }
            }
        }

        // Turn effects into events, and buffer them
        for effect in effects {
            self.events.push_back(effect.into());
        }

        // Return new event if there is some
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
