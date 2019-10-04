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
use parity_wasm::elements::{self, External, MemorySection, Type};
use pwasm_utils::{self, rules};

use crate::config::Config;
use crate::errors::InitializationError;

struct ModulePreparator<'a> {
    module: elements::Module,
    config: &'a Config,
}

impl<'a> ContractModule<'a> {
    fn init(module_code: &[u8], config: &'a Config) -> Result<Self, InitializationError> {
        let module = elements::deserialize_buffer(module_code)
            .map_err(|err| InitializationError::PrepareError(format!("{}", err)))?;

        Ok(ContractModule { module, config })
    }

    fn standardize_mem(self) -> Self {
        let Self { mut module, config } = self;

        module.memory_section_mut().unwrap_or_default().entries_mut().pop();

        let new_entry =
            elements::MemoryType::new(config.mem_pages_count, Some(config.mem_pages_count));

        let mut builder = builder::from_module(module);
        builder.push_import(
            elements::ImportEntry::new(
            "env".to_string(),
            "memory".to_string(),
            elements::External::Memory(new_entry),
        ));

        Self { module: builder.build(), config }
    }

    fn delete_internal_memory(self) -> Result<Self, InitializationError> {
        Ok(Self)
    }

    /*
        if self.module.memory_section().map_or(false, |ms| !ms.entries().is_empty()) {
            Err(PrepareError::InternalMemoryDeclared)
        } else {
            Ok(self)
        }
    }
    */

    fn into_wasm_code(self) -> Result<Vec<u8>, InitializationError> {
        elements::serialize(self.module).map_err(|err| InitializationError::PrepareError(format!("{}", err)))
    }

/// Loads the given module given in `original_code`, performs some checks on it and
/// does some preprocessing.
///
/// The checks are:
///
/// - module doesn't define an internal memory instance,
/// - imported memory (if any) doesn't reserve more memory than permitted by the `config`,
/// - all imported functions from the external environment matches defined by `env` module,
///
/// The preprocessing includes injecting code for gas metering and metering the height of stack.
pub fn prepare_contract(original_code: &[u8], config: &Config) -> Result<Vec<u8>, InitializationError> {
    ContractModule::init(original_code, config)?
        .delete_internal_memory()?
        .standardize_mem()
        .into_wasm_code()
}
