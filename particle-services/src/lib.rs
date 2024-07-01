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
