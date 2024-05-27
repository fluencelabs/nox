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
