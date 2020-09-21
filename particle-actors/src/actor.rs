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
#![allow(dead_code)]

use crate::actor::VmState::{Executing, Idle};
use crate::config::ActorConfig;
use async_std::pin::Pin;
use async_std::task;
use fluence_app_service::{AppService, AppServiceError, RawModulesConfig};
use futures::future::BoxFuture;
use futures::Future;
use libp2p::PeerId;
use particle_protocol::Particle;
use serde_json::json;
use std::collections::VecDeque;
use std::mem;
use std::path::PathBuf;
use std::task::{Context, Poll, Waker};

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub struct FutResult {
    vm: AppService,
    effect: ActorEvent,
}

pub enum ActorEvent {
    Forward { particle: Particle, target: PeerId },
}

enum VmState {
    Idle(AppService),
    Executing(Fut),
    Polling,
}

pub struct Actor {
    vm: VmState,
    particle: Particle,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
}

impl Actor {
    pub fn new(config: ActorConfig, particle: Particle) -> Result<Self, AppServiceError> {
        let vm = Self::create_vm(config, particle.id.clone(), particle.init_peer_id.clone())?;
        Ok(Self {
            vm: Idle(vm),
            particle,
            mailbox: <_>::default(),
            waker: <_>::default(),
        })
    }

    pub fn particle(&self) -> &Particle {
        &self.particle
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ActorEvent> {
        let waker = cx.waker().clone();
        self.waker = Some(waker.clone());

        let vm = mem::replace(&mut self.vm, VmState::Polling);
        match vm {
            Idle(vm) => self.vm = self.execute_next(vm, waker),
            VmState::Executing(mut fut) => {
                if let Poll::Ready(FutResult { vm, effect }) = Pin::new(&mut fut).poll(cx) {
                    self.vm = self.execute_next(vm, waker);
                    return Poll::Ready(effect);
                }
            }
            VmState::Polling => unreachable!("polling race"),
        }

        Poll::Pending
    }

    // TODO: check resolve works fine with new providers
    // TODO: remove provider on client disconnect

    fn create_vm(
        config: ActorConfig,
        particle_id: String,
        owner_id: PeerId,
    ) -> Result<AppService, AppServiceError> {
        let to_string =
            |path: &PathBuf| -> Option<_> { path.to_string_lossy().into_owned().into() };

        let modules = RawModulesConfig {
            modules_dir: to_string(&config.modules_dir),
            service_base_dir: to_string(&config.workdir),
            module: vec![config.stepper_config],
            default: None,
        };

        let mut envs = config.service_envs;
        envs.push(format!("owner_id={}", owner_id));
        /*
        if let Some(owner_pk) = owner_pk {
            envs.push(format!("owner_pk={}", owner_pk));
        };
        */
        log::info!("Creating service {}, envs: {:?}", particle_id, envs);

        AppService::new(modules, &particle_id, envs)

        // TODO: Save created service to disk, so it is recreated on restart
        // Self::persist_service(&config.services_dir, &service_id, &blueprint_id)?;
    }

    fn execute_next(&mut self, vm: AppService, waker: Waker) -> VmState {
        match self.mailbox.pop_front() {
            Some(p) => Executing(Self::execute(p, vm, waker)),
            None => Idle(vm),
        }
    }

    fn execute(particle: Particle, mut vm: AppService, waker: Waker) -> Fut {
        Box::pin(task::spawn_blocking(move || {
            vm.call("stepper", "step", json!(particle.clone()), <_>::default());

            let effect = ActorEvent::Forward {
                target: particle.init_peer_id.clone(),
                particle,
            };

            waker.wake();

            FutResult { vm, effect }
        }))
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}
