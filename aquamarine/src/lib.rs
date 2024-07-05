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

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod actor;
mod aquamarine;
mod command;
mod config;
mod deadline;
mod error;
mod log;
mod particle_data_store;
mod particle_executor;
mod particle_functions;
mod plumber;
mod spawner;

mod aqua_runtime;

mod particle_effects;

mod health;
mod vm_pool;

pub use crate::aqua_runtime::AquaRuntime;
pub use crate::aquamarine::{AquamarineApi, AquamarineBackend};
pub use crate::config::{DataStoreConfig, VmConfig, VmPoolConfig};
pub use crate::particle_effects::{InterpretationStats, ParticleEffects, RemoteRoutingEffects};
pub type AVMRunner = avm_server::avm_runner::AVMRunner<WasmtimeWasmBackend>;
pub use error::AquamarineApiError;
pub use marine_wasmtime_backend::WasmtimeWasmBackend;
pub use particle_data_store::{DataStoreError, ParticleDataStore};
pub use particle_services::WasmBackendConfig;
pub use plumber::Plumber;
