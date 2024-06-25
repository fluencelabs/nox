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
