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

use crate::wasm_backend_config::WasmBackendConfig;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Clone, Default, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct AVMConfig {
    /// Maximum heap size in bytes available for an interpreter instance.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub aquavm_heap_size_limit: Option<bytesize::ByteSize>,

    /// Maximum AIR size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub air_size_limit: Option<bytesize::ByteSize>,

    /// Maximum particle size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub particle_size_limit: Option<bytesize::ByteSize>,

    /// Maximum service call result size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub call_result_size_limit: Option<bytesize::ByteSize>,

    /// Hard limit AquaVM behavior control knob.
    #[serde(default)]
    pub hard_limit_enabled: bool,

    #[serde(default)]
    pub wasm_backend: WasmBackendConfig,
}
