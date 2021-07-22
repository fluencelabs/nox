/*
 * Copyright 2021 Fluence Labs Limited
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

pub use modules::blueprint::Blueprint;
pub use modules::dependencies::*;
pub use modules::dependency::Dependency;
pub use modules::file_names::*;
pub use modules::fixture::{load_module, module_config};
pub use modules::hash::Hash;

mod modules {
    pub mod blueprint;
    pub mod dependencies;
    pub mod dependency;
    pub mod file_names;
    pub mod fixture;
    pub mod hash;
}
