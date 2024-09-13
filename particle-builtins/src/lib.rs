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

#![feature(stmt_expr_attributes)]
#![feature(try_trait_v2)]
#![feature(try_blocks)]
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

pub use builtins::{Builtins, BuiltinsConfig, CustomService};
pub use identify::{NodeInfo, PortInfo, VmInfo};
pub use outcome::{ok, wrap, wrap_unit};
pub use particle_services::ParticleAppServicesConfig;
mod builtins;
mod debug;
mod error;
mod func;
mod identify;
mod json;
mod math;
mod outcome;
mod particle_function;
