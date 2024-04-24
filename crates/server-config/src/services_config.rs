use crate::wasm_backend_config::WasmBackendConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesConfig {
    pub wasm_backend: WasmBackendConfig,
}
