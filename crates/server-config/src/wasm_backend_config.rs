/*
 * Copyright 2024 Fluence DAO
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

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBackendConfig {
    /// Configures whether DWARF debug information will be emitted during compilation.
    pub debug_info: bool,
    /// Configures whether the errors from the VM should collect the wasm backtrace and parse debug info.
    pub wasm_backtrace: bool,
    /// Configures the size of the stacks used for asynchronous execution.
    pub async_wasm_stack: bytesize::ByteSize,
    /// Configures the maximum amount of stack space available for executing WebAssembly code.
    pub max_wasm_stack: bytesize::ByteSize,
    /// Enables the epoch interruption mechanism
    #[serde(with = "humantime_serde")]
    pub epoch_interruption_duration: Option<Duration>,
}

impl Default for WasmBackendConfig {
    fn default() -> Self {
        Self {
            debug_info: true,
            wasm_backtrace: true,
            async_wasm_stack: bytesize::ByteSize::mb(4),
            max_wasm_stack: bytesize::ByteSize::mb(2),
            epoch_interruption_duration: Some(Duration::from_secs(1)),
        }
    }
}
