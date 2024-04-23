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

#![feature(try_blocks, result_option_inspect)]
#![feature(hash_extract_if)]
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

pub use fluence_app_service::{IType, IValue};

pub use app_services::ParticleAppServices;
pub use app_services::ServiceType;

pub use crate::error::ServiceError;

mod app_services;
mod error;
mod health;
mod persistence;

mod config;

pub use app_services::ServiceInfo;
pub use config::ParticleAppServicesConfig;
pub use config::WasmBackendConfig;
pub use types::peer_scope::PeerScope;
