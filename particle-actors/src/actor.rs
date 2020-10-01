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

use crate::actor::VmState::{Executing, Idle};
use crate::config::ActorConfig;
use crate::invoke::parse_outcome;
use crate::plumber::ClosureDescriptor;

use aquamarine_vm::{AquamarineVM, AquamarineVMConfig, AquamarineVMError};
use particle_protocol::Particle;

use async_std::{pin::Pin, task};
use futures::{future::BoxFuture, Future, FutureExt};
use libp2p::PeerId;
use serde_json::json;
use std::{
    collections::VecDeque,
    mem,
    task::{Context, Poll, Waker},
};

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub struct FutResult {
    vm: AquamarineVM,
    effects: Vec<ActorEvent>,
}

pub enum ActorEvent {
    Forward { particle: Particle, target: PeerId },
}

enum VmState {
    Idle(AquamarineVM),
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
    pub fn new(
        config: ActorConfig,
        particle: Particle,
        host_closure: ClosureDescriptor,
    ) -> Result<Self, AquamarineVMError> {
        log::info!("creating vm");
        let vm = Self::create_vm(config, host_closure)?;
        log::info!("vm created");
        let mut this = Self {
            vm: Idle(vm),
            particle: particle.clone(),
            mailbox: <_>::default(),
            waker: <_>::default(),
        };

        this.ingest(particle);

        Ok(this)
    }

    pub fn particle(&self) -> &Particle {
        &self.particle
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Vec<ActorEvent>> {
        let waker = cx.waker().clone();
        self.waker = Some(waker.clone());

        let vm = mem::replace(&mut self.vm, VmState::Polling);
        let execute = |vm| self.execute_next(vm, waker);

        let (state, effects) = match vm {
            Idle(vm) => (execute(vm), Poll::Pending),
            Executing(mut fut) => {
                if let Poll::Ready(FutResult { vm, effects }) = Pin::new(&mut fut).poll(cx) {
                    (execute(vm), Poll::Ready(effects))
                } else {
                    (Executing(fut), Poll::Pending)
                }
            }
            VmState::Polling => unreachable!("polling race"),
        };

        self.vm = state;
        return effects;
    }

    fn create_vm(
        config: ActorConfig,
        host_closure: ClosureDescriptor,
    ) -> Result<AquamarineVM, AquamarineVMError> {
        let config = AquamarineVMConfig {
            current_peer_id: config.current_peer_id.to_string(),
            aquamarine_wasm_path: config.modules_dir.join("aquamarine.wasm"),
            call_service: host_closure(),
        };
        AquamarineVM::new(config)
    }

    fn execute_next(&mut self, vm: AquamarineVM, waker: Waker) -> VmState {
        match self.mailbox.pop_front() {
            Some(p) => Executing(Self::execute(p, vm, waker)),
            None => Idle(vm),
        }
    }

    fn execute(particle: Particle, mut vm: AquamarineVM, waker: Waker) -> Fut {
        log::info!("Scheduling particle for execution {:?}", particle.id);
        task::spawn_blocking(move || {
            let args = json!({
                "init_user_id": particle.init_peer_id.to_string(),
                "aqua": particle.script,
                "data": particle.data.to_string()
            });
            log::info!("Executing particle {}, args {:?}", particle.id, args);
            let result = vm.call(args);

            if let Err(err) = &result {
                log::warn!("Error executing particle {}: {:?}", particle.id, err)
            }

            let effects = match parse_outcome(result) {
                Ok((data, targets)) => {
                    let mut particle = particle;
                    particle.data = data;
                    targets
                        .into_iter()
                        .map(|target| ActorEvent::Forward {
                            particle: particle.clone(),
                            target,
                        })
                        .collect::<Vec<_>>()
                }
                Err(err) => {
                    let mut particle = particle;
                    let error = format!("{:?}", err);
                    if let Some(map) = particle.data.as_object_mut() {
                        map.insert("protocol!error".to_string(), json!(error));
                    } else {
                        particle.data = json!({"protocol!error": error, "data": particle.data})
                    }
                    // Return error to the init peer id
                    vec![ActorEvent::Forward {
                        target: particle.init_peer_id.clone(),
                        particle,
                    }]
                }
            };

            log::debug!("Parsed result on particle");

            waker.wake();

            FutResult { vm, effects }
        })
        .boxed()
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

impl std::fmt::Display for VmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Idle(_) => "idle",
            Executing(_) => "executing",
            VmState::Polling => "polling",
        };
        write!(f, "{}", str)
    }
}
