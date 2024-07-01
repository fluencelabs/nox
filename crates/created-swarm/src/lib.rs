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

#![feature(try_blocks)]
#![feature(async_closure)]
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

mod services;
mod swarm;

pub use crate::services::*;
pub use crate::swarm::*;

pub use server_config::system_services_config;
pub use server_config::ChainConfig;

pub use particle_args::{Args, JError};
pub use particle_execution::{FunctionOutcome, ParticleParams};

pub use fluence_app_service;
pub use fluence_keypair;
pub use fluence_spell_dtos;
pub use system_services;
