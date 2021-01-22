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
use futures::{future::BoxFuture, FutureExt};
use std::{
    collections::VecDeque,
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
    creating_vms: Option<Vec<BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>>>>,
    host_closure: ClosureDescriptor,
    config: VmPoolConfig,
}

impl VmPool {
    /// Creates `VmPool` and starts background tasks creating `config.pool_size` number of VMs
    pub fn new(config: VmPoolConfig, host_closure: ClosureDescriptor) -> Self {
        Self {
            vms: <_>::default(),
            creating_vms: None,
            host_closure,
            config,
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
        let creating_vms = match &mut self.creating_vms {
            None => {
                log::info!(target: "debug_vms", "Started creating VMs");
                self.creating_vms = Some(
                    (0..self.config.pool_size)
                        .map(|_| {
                            let config = self.config.clone();
                            let host_closure = self.host_closure.clone();
                            let waker = cx.waker().clone();

                            create_vm(config, host_closure, waker)
                        })
                        .collect(),
                );
                self.creating_vms.as_mut().unwrap()
            }
            Some(ref mut vms) => vms,
        };

        let mut wake = false;

        let mut i = 0;
        while i < creating_vms.len() {
            let vms = &mut self.vms;
            let fut = &mut creating_vms[i];
            if let Poll::Ready(vm) = fut.poll_unpin(cx) {
                // Remove completed future
                creating_vms.remove(i);
                if creating_vms.is_empty() {
                    log::info!("All stepper VMs created.")
                }

                // Put created vm to self.vms
                match vm {
                    Ok(vm) => vms.push_back(vm),
                    Err(err) => log::error!("Failed to create vm: {:?}", err), // TODO: don't panic
                }

                wake = true;
            }
            i += 1;
        }

        if wake {
            cx.waker().wake_by_ref()
        }
    }
}

/// Creates `AquamarineVM` in background (on blocking threadpool)
fn create_vm(
    config: VmPoolConfig,
    host_closure: ClosureDescriptor,
    waker: Waker,
) -> BoxFuture<'static, Result<AquamarineVM, AquamarineVMError>> {
    task::spawn_blocking(move || {
        let config = AquamarineVMConfig {
            current_peer_id: config.current_peer_id.to_string(),
            aquamarine_wasm_path: config.air_interpreter,
            particle_data_store: config.particles_dir,
            call_service: host_closure(),
            logging_mask: i32::max_value(),
        };
        let vm = AquamarineVM::new(config);
        waker.wake();
        vm
    })
    .boxed()
}
