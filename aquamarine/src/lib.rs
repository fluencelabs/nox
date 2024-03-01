/*
 * Copyright 2020 Fluence Labs Limited
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

mod vm_pool;
mod health;

pub use crate::aqua_runtime::AquaRuntime;
pub use crate::aquamarine::{AquamarineApi, AquamarineBackend};
pub use crate::config::{VmConfig, VmPoolConfig, DataStoreConfig};
pub use crate::particle_effects::{InterpretationStats, ParticleEffects, RemoteRoutingEffects};
pub use avm_server::avm_runner::AVMRunner;
pub use error::AquamarineApiError;
pub use particle_data_store::{DataStoreError, ParticleDataStore};
pub use plumber::Plumber;
