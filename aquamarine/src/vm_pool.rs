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

use std::error::Error;
use std::fmt::Debug;
use std::task::{Context, Poll};

use futures::{future::BoxFuture, FutureExt};
use health::HealthCheckRegistry;
use tokio::task::JoinError;

use peer_metrics::VmPoolMetrics;

use crate::aqua_runtime::AquaRuntime;
use crate::health::VMPoolHealth;

type RuntimeF<RT> = BoxFuture<'static, Result<RT, CreateAVMError>>;

#[derive(Debug)]
enum CreateAVMError {
    AVMError(Box<dyn Error + Send + Sync + 'static>),
    JoinError(JoinError),
}

/// Pool that owns and manages aquamarine stepper VMs
/// VMs are created asynchronously after `VmPool` creation
/// Futures representing background VM creation are stored in `VmPool::creating_runtimes`
/// Created vms are moved to `VmPool::runtimes`
///
/// Main API consists of `VmPool::get_vm` and `VmPool::put_vm`.
/// API allows taking VM for execution (via `get_vm`), and then it is expected that VM is
/// returned back via `put_vm`.
/// It is also expected that `VmPool::poll` is called periodically.
pub struct VmPool<RT: AquaRuntime> {
    runtimes: Vec<Option<RT>>,
    creating_runtimes: Option<Vec<(usize, RuntimeF<RT>)>>,
    runtime_config: RT::Config,
    pool_size: usize,
    metrics: Option<VmPoolMetrics>,
    health: Option<VMPoolHealth>,
}

//TODO:check
impl<RT: AquaRuntime> VmPool<RT> {
    /// Creates `VmPool` and starts background tasks creating `config.pool_size` number of VMs
    pub fn new(
        pool_size: usize,
        runtime_config: RT::Config,
        metrics: Option<VmPoolMetrics>,
        health_registry: Option<&mut HealthCheckRegistry>,
    ) -> Self {
        let health = health_registry.map(|registry| {
            let health = VMPoolHealth::new(pool_size);
            registry.register("vm_pool", health.clone());
            health
        });

        let mut this = Self {
            runtimes: Vec::with_capacity(pool_size),
            creating_runtimes: None,
            runtime_config,
            pool_size,
            metrics,
            health,
        };

        this.runtimes.resize_with(pool_size, || None);
        this.meter(|m| m.set_pool_size(pool_size));

        this
    }

    fn meter<U, FF: Fn(&mut VmPoolMetrics) -> U>(&mut self, f: FF) {
        self.metrics.as_mut().map(f);
    }

    /// Number of currently unused vms
    pub fn free_vms(&self) -> usize {
        self.runtimes.len()
    }

    /// Takes VM from pool
    pub fn get_vm(&mut self) -> Option<(usize, RT)> {
        let runtimes = self.runtimes.iter_mut();
        let vm = runtimes
            .enumerate()
            .find_map(|(idx, vm)| vm.take().map(|vm| (idx, vm)));

        let free_vms_count = self.runtimes.iter().filter(|vm| vm.is_some()).count();
        self.meter(|m| {
            m.get_vm.inc();

            if vm.is_none() {
                m.no_free_vm.inc();
            }
            m.free_vms.set(free_vms_count as i64);
        });

        vm
    }

    /// Puts VM back to the pool
    pub fn put_vm(&mut self, id: usize, vm: RT) {
        debug_assert!(
            self.runtimes[id].is_none(),
            "put_vm must never happen before get_vm"
        );
        let memory_stats = vm.memory_stats();
        self.runtimes[id] = Some(vm);

        let free_vms_count = self.runtimes.iter().filter(|vm| vm.is_some()).count();
        self.meter(|m| {
            m.put_vm.inc();
            m.free_vms.set(free_vms_count as i64);
            m.measure_memory(id, memory_stats.memory_size as u64);
            // TODO: measure max memory
        });
    }

    pub fn recreate_avm(&mut self, id: usize, cx: &mut Context<'_>) {
        if self.creating_runtimes.is_none() {
            log::error!(
                "Attempt to recreate an AVM before initialization (self.creating_runtimes is None), ignoring"
            );
            return;
        }

        let avm_f = self.create_avm(cx);
        if let Some(creating_vms) = self.creating_runtimes.as_mut() {
            creating_vms.push((id, avm_f))
        }
    }

    fn create_avm(&self, cx: &mut Context<'_>) -> RuntimeF<RT> {
        let config = self.runtime_config.clone();
        let waker = cx.waker().clone();

        async {
            let task_result =
                tokio::task::spawn_blocking(|| RT::create_runtime(config, waker)).await; //TODO: move waker outside create runtime
            match task_result {
                Ok(joined_res) => joined_res.map_err(|e| CreateAVMError::AVMError(Box::new(e))),
                Err(e) => Err(CreateAVMError::JoinError(e)),
            }
        }
        .boxed()
    }

    /// Moves created VMs from `creating_vms` to `vms`
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        let creating_vms = match &mut self.creating_runtimes {
            None => {
                self.creating_runtimes = Some(
                    (0..self.pool_size)
                        .map(|id| (id, self.create_avm(cx)))
                        .collect(),
                );
                self.creating_runtimes.as_mut().unwrap()
            }
            Some(ref mut vms) => vms,
        };

        let mut wake = false;

        let mut fut_index = 0;
        while fut_index < creating_vms.len() {
            let vms = &mut self.runtimes;
            let idx_fut = &mut creating_vms[fut_index];
            let id = idx_fut.0;
            let fut = &mut idx_fut.1;
            if let Poll::Ready(vm) = fut.poll_unpin(cx) {
                // Remove completed future
                creating_vms.remove(fut_index);
                if creating_vms.is_empty() {
                    tracing::info!("All {} AquaVMs created.", self.pool_size)
                }

                // Put created vm to self.vms
                match vm {
                    Ok(vm) => {
                        vms[id] = Some(vm);
                        if let Some(h) = self.health.as_ref() {
                            h.increment_count()
                        }
                    }
                    Err(err) => {
                        tracing::error!("Failed to create vm: {:?}", err)
                    } // TODO: don't panic
                }

                wake = true;
            }
            fut_index += 1;
        }

        if wake {
            cx.waker().wake_by_ref()
        }
    }
}
