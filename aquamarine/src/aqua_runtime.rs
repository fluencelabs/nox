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

use aquamarine_vm::{AquamarineVM, AquamarineVMConfig, AquamarineVMError, InterpreterOutcome};
use host_closure::ClosureDescriptor;

use crate::config::VmConfig;
use async_std::task;
use futures::{future::BoxFuture, FutureExt};
use std::{error::Error, task::Waker};

pub trait AquaRuntime: Sized {
    type Config: Clone;
    type Error: Error;

    fn create_runtime(
        config: Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>>;
    fn call(
        &mut self,
        init_user_id: impl Into<String>,
        aqua: impl Into<String>,
        data: impl Into<Vec<u8>>,
        particle_id: impl Into<String>,
    ) -> Result<InterpreterOutcome, Self::Error>;
}

impl AquaRuntime for AquamarineVM {
    type Config = (VmConfig, ClosureDescriptor);
    type Error = AquamarineVMError;

    /// Creates `AquamarineVM` in background (on blocking threadpool)
    fn create_runtime(
        (config, host_closure): Self::Config,
        waker: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>> {
        task::spawn_blocking(move || {
            let config = AquamarineVMConfig {
                current_peer_id: config.current_peer_id.to_string(),
                aquamarine_wasm_path: config.air_interpreter,
                particle_data_store: config.particles_dir,
                call_service: host_closure(),
                logging_mask: i32::MAX,
            };
            let vm = AquamarineVM::new(config);
            waker.wake();
            vm
        })
        .boxed()
    }

    #[inline]
    fn call(
        &mut self,
        init_user_id: impl Into<String>,
        aqua: impl Into<String>,
        data: impl Into<Vec<u8>>,
        particle_id: impl Into<String>,
    ) -> Result<InterpreterOutcome, Self::Error> {
        AquamarineVM::call(self, init_user_id, aqua, data, particle_id)
    }
}
