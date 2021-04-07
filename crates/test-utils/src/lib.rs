#![feature(once_cell)]
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
#![warn(rust_2018_idioms)]
#![allow(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns,
    unreachable_code
)]

#[macro_use]
extern crate fstrings;

mod connected_client;
mod connection;
mod easy_vm;
mod local_vm;
mod misc;
mod service;
mod singleton_vm;
mod utils;

pub use connected_client::*;
pub use connection::*;
pub use easy_vm::*;
pub use local_vm::*;
pub use misc::*;
pub use service::*;
pub use utils::*;
