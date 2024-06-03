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

#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns,
    unreachable_code
)]
#![feature(try_blocks)]

pub use api::ConnectionPoolApi;
// to be available in benchmarks
pub use api::Command;
pub use behaviour::ConnectionPoolBehaviour;

pub use crate::connection_pool::ConnectionPoolT;
pub use crate::connection_pool::LifecycleEvent;

mod api;
mod behaviour;
mod connection_pool;
