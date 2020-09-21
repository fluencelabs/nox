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

use crate::config::ActorConfig;
use either::Either::{self, Left, Right};
use fluence_app_service::{AppService, AppServiceError, RawModulesConfig};
use futures::future::BoxFuture;
use libp2p::PeerId;
use particle_protocol::Particle;
use std::collections::VecDeque;
use std::path::PathBuf;
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

pub struct Actor {
    vm: AppService,
    particle: Particle,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
}

impl Actor {
    pub fn new(config: ActorConfig, particle: Particle) -> Result<Self, AppServiceError> {
        let vm = Self::create_vm(config, particle.id.clone(), particle.init_peer_id.clone())?;
        Ok(Self {
            vm,
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
        self.waker = Some(cx.waker().clone());

        // // Execute next particle or keep waiting for a current work to complete
        // self.vm_or_work = Some(vm_or_work.either(|vm| self.execute_next(vm), |f| Right(f)));
        //
        // return effect;

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
