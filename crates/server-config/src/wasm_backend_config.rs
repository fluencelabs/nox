use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBackendConfig {
    /// Configures whether DWARF debug information will be emitted during compilation.
    pub debug_info: bool,
    /// Configures whether the errors from the VM should collect the wasm backtrace and parse debug info.
    pub wasm_backtrace: bool,
    /// Configures the size of the stacks used for asynchronous execution.
    pub async_wasm_stack: usize,
    /// Configures the maximum amount of stack space available for executing WebAssembly code.
    pub max_wasm_stack: usize,
    /// Enables the epoch interruption mechanism
    #[serde(with = "humantime_serde")]
    pub epoch_interruption_duration: Option<Duration>,
}

impl Default for WasmBackendConfig {
    fn default() -> Self {
        Self {
            debug_info: true,
            wasm_backtrace: true,
            async_wasm_stack: 2 * 1024 * 1024,
            max_wasm_stack: 2 * 1024 * 1024,
            epoch_interruption_duration: Some(Duration::from_secs(1)),
        }
    }
}
