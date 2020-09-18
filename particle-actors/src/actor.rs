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

use async_std::task;
use either::Either::{self, Left, Right};
use fluence_app_service::{AppService, AppServiceError, RawModuleConfig, RawModulesConfig};
use futures::future::BoxFuture;
use futures::Future;
use libp2p::PeerId;
use particle_protocol::Particle;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub enum FutResult {
    Created(AppService),
    CreationFailed(AppServiceError),
    Effect(AppService, ActorEvent),
}

pub enum ActorEvent {
    Forward { particle: Particle, target: PeerId },
}

pub struct ActorConfig {
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub service_envs: Vec<String>,
    /// Working dir for services
    pub workdir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running services
    pub services_dir: PathBuf,
    /// Module config for the stepper
    pub stepper_config: RawModuleConfig,
}

struct Actor {
    vm_or_work: Option<Either<AppService, Fut>>,
    particle: Particle,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
}

impl Actor {
    pub fn new(config: ActorConfig, particle: Particle) -> Self {
        let fut = Self::create_vm(config, particle.id.clone(), particle.init_peer_id.clone());
        Self {
            vm_or_work: Some(Right(fut)),
            particle,
            mailbox: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ActorEvent> {
        self.waker = Some(cx.waker().clone());

        let vm_or_work = match self.vm_or_work.take() {
            Some(v) => v,
            // None means vm creation failed, need to always return an error, where?
            None => return Poll::Pending,
        };

        let mut effect = Poll::Pending;
        let vm_or_work = match vm_or_work {
            Right(mut future) => match Pin::new(&mut future).poll(cx) {
                // Future hasn't completed yet
                Poll::Pending => Right(future),
                // Failed to create VM, will leave self.vm_or_work empty
                Poll::Ready(FutResult::CreationFailed(_)) => return Poll::Pending,
                // Execution completed, will return result
                Poll::Ready(FutResult::Effect(vm, p)) => {
                    effect = Poll::Ready(p);
                    Left(vm)
                }
                // VM created
                Poll::Ready(FutResult::Created(vm)) => Left(vm),
            },
            Left(vm) => Left(vm),
        };

        // Execute next particle or keep waiting for a current work to complete
        self.vm_or_work = Some(vm_or_work.either(|vm| self.execute_next(vm), |f| Right(f)));

        return effect;
    }

    // TODO: check resolve works fine with new providers
    // TODO: remove provider on client disconnect

    fn create_vm(config: ActorConfig, particle_id: String, owner_id: PeerId) -> Fut {
        Box::pin(task::spawn_blocking(move || {
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

            match AppService::new(modules, &particle_id, envs) {
                Ok(vm) => FutResult::Created(vm),
                Err(err) => FutResult::CreationFailed(err),
            }

            // TODO: Save created service to disk, so it is recreated on restart
            // Self::persist_service(&config.services_dir, &service_id, &blueprint_id)?;
        }))
    }

    fn execute_next(&mut self, vm: AppService) -> Either<AppService, Fut> {
        match self.mailbox.pop_front() {
            Some(p) => Right(Self::execute(p, vm)),
            None => Left(vm),
        }
    }

    fn execute(_particle: Particle, _vm: AppService) -> Fut {
        Box::pin(async { unimplemented!() })
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.clone() {
            waker.clone().wake()
        }
    }
}
