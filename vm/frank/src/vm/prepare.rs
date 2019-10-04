/*
 * Copyright 2019 Fluence Labs Limited
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

// Similar to
// https://github.com/paritytech/substrate/blob/master/srml/contracts/src/wasm/prepare.rs
// https://github.com/nearprotocol/nearcore/blob/master/runtime/near-vm-runner/src/prepare.rs

use parity_wasm::builder;
use parity_wasm::elements;

use crate::vm::config::Config;
use crate::vm::errors::InitializationError;

struct ModulePreparator {
    module: elements::Module,
}

impl<'a> ModulePreparator {
    fn init(module_code: &[u8]) -> Result<Self, InitializationError> {
        let module = elements::deserialize_buffer(module_code)
            .map_err(|err| InitializationError::PrepareError(format!("{}", err)))?;

        Ok(Self { module })
    }

    fn set_mem_pages_count(self, mem_pages_count: u32) -> Self {
        let builder = builder::from_module(self.module);
        let memory_builder = builder.memory();
        let builder = memory_builder.with_min(mem_pages_count).with_max(Some(mem_pages_count)).build();

        Self {
            module: builder.build(),
        }
    }

    fn to_wasm_code(self) -> Result<Vec<u8>, InitializationError> {
        elements::serialize(self.module)
            .map_err(|err| InitializationError::PrepareError(format!("{}", err)))
    }
}

pub fn prepare_module(module: &[u8], config: &Config) -> Result<Vec<u8>, InitializationError> {
    ModulePreparator::init(module)?
        .set_mem_pages_count(config.mem_pages_count as _)
        .to_wasm_code()
}
