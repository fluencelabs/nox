/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
