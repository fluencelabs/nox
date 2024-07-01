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
#![feature(extend_one)]
pub use sorcerer::Sorcerer;
pub use spell_builtins::{get_spell_info, install_spell, remove_spell, SpellInfo};

#[macro_use]
extern crate fstrings;

mod error;
mod script_executor;
mod sorcerer;
mod spell_builtins;
mod utils;
mod worker_builins;
