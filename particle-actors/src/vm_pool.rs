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

use crate::VmPoolConfig;

use aquamarine_vm::{AquamarineVM, AquamarineVMConfig, AquamarineVMError};
use host_closure::ClosureDescriptor;

use async_std::task;
use futures::{future::BoxFuture, Future, FutureExt};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

/// Pool that owns and manages aquamarine stepper VMs
/// VMs are created asynchronously after `VmPool` creation
/// Futures representing background VM creation are stored in `VmPool::creating_vms`
/// Created vms are moved to `VmPool::vms`
///
/// Main API consists of `VmPool::get_vm` and `VmPool::put_vm`.
/// API allows taking VM for execution (via `get_vm`), and then it is expected that VM is
/// returned back via `put_vm`.
/// It is also expected that `VmPool::poll` is called periodically.
pub struct VmPool {
    vms: VecDeque<AquamarineVM>,
    creating_vms: Vec<BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>>>,
    waker: Arc<RwLock<Option<Waker>>>,
}

impl VmPool {
    /// Creates `VmPool` and starts background tasks creating `config.pool_size` number of VMs
    pub fn new(config: VmPoolConfig, host_closure: ClosureDescriptor) -> Self {
        let waker: Arc<RwLock<Option<Waker>>> = <_>::default();
        let creating_vms = (0..config.pool_size)
            .map(|_| {
                let config = config.clone();
                let host_closure = host_closure.clone();
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

    /// Takes VM from pool
    pub fn get_vm(&mut self) -> Option<AquamarineVM> {
        self.vms.pop_front()
    }

    /// Puts VM back to the pool
    pub fn put_vm(&mut self, vm: AquamarineVM) {
        self.vms.push_front(vm)
    }

    /// Moves created VMs from `creating_vms` to `vms`
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

/// Creates `AquamarineVM` in background (on blocking threadpool)
fn create_vm(
    config: VmPoolConfig,
    host_closure: ClosureDescriptor,
    waker: Arc<RwLock<Option<Waker>>>,
) -> BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>> {
    task::spawn_blocking(move || {
        log::info!("preparing vm config");
        let config = AquamarineVMConfig {
            current_peer_id: config.current_peer_id.to_string(),
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
