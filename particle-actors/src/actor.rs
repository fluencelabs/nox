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

use fluence_app_service::AppService;
use futures::future::BoxFuture;
use libp2p::PeerId;
use particle_protocol::Particle;
use std::collections::VecDeque;
use std::task::{Poll, Waker};

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub enum FutResult {
    Created(AppService),
    Effect,
}

pub enum ActorEvent {
    Forward { particle: Particle, target: PeerId },
}

struct Actor {
    vm: Option<AppService>,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
}

impl Actor {
    pub fn new() -> Self {
        Self {
            vm: <_>::default(),
            mailbox: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    pub fn poll(&mut self, waker: Waker) -> Poll<ActorEvent> {
        self.waker = Some(waker);

        match self.vm {
            None => Self::create_vm()
            Some(_) => {}
        }
    }

    fn create_vm() -> Fut {
        unimplemented!()
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.clone() {
            waker.clone()
        }
    }
}
