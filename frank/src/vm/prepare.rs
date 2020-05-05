/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

// Similar to
// https://github.com/paritytech/substrate/blob/master/srml/contracts/src/wasm/prepare.rs
// https://github.com/nearprotocol/nearcore/blob/master/runtime/near-vm-runner/src/prepare.rs

use parity_wasm::builder;
use parity_wasm::elements;

use crate::vm::config::Config;
use crate::vm::errors::InitializationError;
use parity_wasm::elements::{MemorySection, MemoryType};

struct ModulePreparator {
    module: elements::Module,
}

impl<'a> ModulePreparator {
    fn init(module_code: &[u8]) -> Result<Self, InitializationError> {
        let module = elements::deserialize_buffer(module_code)?;

        Ok(Self { module })
    }

    fn set_mem_pages_count(self, mem_pages_count: u32) -> Self {
        let Self { mut module } = self;

        // At now, there is could be only one memory section, so
        // it needs just to extract previous initial page count,
        // delete an old entry and add create a new one with updated limits
        let mem_initial = match module.memory_section_mut() {
            Some(section) => match section.entries_mut().pop() {
                Some(entry) => entry.limits().initial(),
                None => 0,
            },
            None => 0,
        };

        let memory_entry = MemoryType::new(mem_initial, Some(mem_pages_count));
        let mut default_mem_section = MemorySection::default();

        module
            .memory_section_mut()
            .unwrap_or_else(|| &mut default_mem_section)
            .entries_mut()
            .push(memory_entry);

        let builder = builder::from_module(module);

        Self {
            module: builder.build(),
        }
    }

    fn into_wasm(self) -> Result<Vec<u8>, InitializationError> {
        elements::serialize(self.module).map_err(Into::into)
    }
}

/// Prepares a Wasm module:
///   - set memory page count
pub fn prepare_module(module: &[u8], config: &Config) -> Result<Vec<u8>, InitializationError> {
    ModulePreparator::init(module)?
        .set_mem_pages_count(config.mem_pages_count as _)
        .into_wasm()
}
