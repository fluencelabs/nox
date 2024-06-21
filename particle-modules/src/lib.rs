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
#![feature(assert_matches)]
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

#[macro_use]
extern crate fstrings;

mod error;
mod files;
mod modules;

pub use error::ModuleError;
pub use files::{load_blueprint, load_module_by_path, load_module_descriptor};
pub use modules::EffectorsMode;
pub use modules::ModuleRepository;

// reexport
pub use fluence_app_service::{
    TomlMarineModuleConfig as ModuleConfig, TomlMarineNamedModuleConfig as NamedModuleConfig,
    TomlWASIConfig as WASIConfig,
};
pub use fs_utils::list_files;
pub use service_modules::AddBlueprint;
