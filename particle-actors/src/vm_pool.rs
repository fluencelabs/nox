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

use crate::ActorConfig;

use aquamarine_vm::{AquamarineVM, AquamarineVMConfig, AquamarineVMError};
use host_closure::ClosureDescriptor;
use particle_protocol::Particle;

use async_std::task;
use futures::{future::BoxFuture, Future, FutureExt};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

pub struct FutResult {}
pub type Fut = BoxFuture<'static, FutResult>;
pub type Closure = Box<dyn Fn(Particle, Waker) -> Fut + Send + Sync + 'static>;

#[derive(Default)]
pub struct VmPool {
    vms: VecDeque<AquamarineVM>,
    creating_vms: Vec<BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>>>,
    waker: Arc<RwLock<Option<Waker>>>,
}

impl VmPool {
    pub fn new(config: ActorConfig, host_closure: ClosureDescriptor) -> Self {
        let waker: Arc<RwLock<Option<Waker>>> = <_>::default();
        let cores = 2; // TODO: gather number of cores from config and/or OS
        log::info!("VmPool::new {:#?}", config);
        let creating_vms = (1..cores)
            .map(|i| {
                log::info!("will create vm, i = {}", i);
                let config = config.clone();
                let host_closure = host_closure.clone();
                // TODO: will captured waker be updated after VmPool::poll?
                let waker = waker.clone();

                create_vm(config, host_closure, waker)
            })
            .collect();

        Self {
            vms: <_>::default(),
            creating_vms,
            waker,
        }
    }

    pub fn get_vm(&mut self) -> Option<AquamarineVM> {
        self.vms.pop_front()
    }

    pub fn put_vm(&mut self, vm: AquamarineVM) {
        self.vms.push_front(vm)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) {
        // Save waker
        self.waker.write().replace(cx.waker().clone());

        for i in 0..self.creating_vms.len() {
            let vms = &mut self.vms;
            let fut = &mut self.creating_vms[i];
            if let Poll::Ready(vm) = Pin::new(fut).poll(cx) {
                // Remove completed future
                self.creating_vms.remove(i);

                // Put created vm to self.vms
                match vm {
                    Ok(vm) => {
                        cx.waker().wake_by_ref(); // TODO: will it wake multiple times?
                        vms.push_back(vm);
                    }
                    Err(err) => log::error!("Failed to create vm: {:?}", err), // TODO: don't panic
                }
            }
        }
    }
}

fn create_vm(
    config: ActorConfig,
    host_closure: ClosureDescriptor,
    waker: Arc<RwLock<Option<Waker>>>,
) -> BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>> {
    task::spawn_blocking(move || {
        log::info!("preparing vm config");
        let config = AquamarineVMConfig {
            current_peer_id: "123".to_string(), // TODO: remove current_peer_id from config?
            aquamarine_wasm_path: config.modules_dir.join("aquamarine.wasm"),
            call_service: host_closure(),
        };
        log::info!("creating vm");
        let vm = AquamarineVM::new(config)?;
        log::info!("vm created");
        if let Some(waker) = waker.read().as_ref() {
            waker.wake_by_ref();
        }
        Ok(vm)
    })
    .boxed()
}
