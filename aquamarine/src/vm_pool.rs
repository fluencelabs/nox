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

use crate::aqua_runtime::AquaRuntime;

use futures::{future::BoxFuture, FutureExt};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
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
pub struct VmPool<RT: AquaRuntime> {
    runtimes: VecDeque<RT>,
    creating_runtimes: Option<Vec<BoxFuture<'static, Result<RT, RT::Error>>>>,
    runtime_config: RT::Config,
    pool_size: usize,
}

impl<RT: AquaRuntime> VmPool<RT> {
    /// Creates `VmPool` and starts background tasks creating `config.pool_size` number of VMs
    pub fn new(pool_size: usize, runtime_config: RT::Config) -> Self {
        Self {
            runtimes: <_>::default(),
            creating_runtimes: None,
            runtime_config,
            pool_size,
        }
    }

    /// Takes VM from pool
    pub fn get_vm(&mut self) -> Option<RT> {
        self.runtimes.pop_front()
    }

    /// Puts VM back to the pool
    pub fn put_vm(&mut self, vm: RT) {
        self.runtimes.push_front(vm)
    }

    /// Moves created VMs from `creating_vms` to `vms`
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        let creating_vms = match &mut self.creating_runtimes {
            None => {
                self.creating_runtimes = Some(
                    (0..self.pool_size)
                        .map(|_| {
                            let config = self.runtime_config.clone();
                            let waker = cx.waker().clone();

                            RT::create_runtime(config, waker)
                        })
                        .collect(),
                );
                self.creating_runtimes.as_mut().unwrap()
            }
            Some(ref mut vms) => vms,
        };

        let mut wake = false;

        let mut i = 0;
        while i < creating_vms.len() {
            let vms = &mut self.runtimes;
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
