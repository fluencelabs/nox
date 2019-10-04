/*
 * Copyright 2018 Fluence Labs Limited
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

//! The main part of Fluence backend SDK. Contains `export_allocator` (is turned on by the
//! `export_allocator` feature), `logger` (is turned on by the `wasm_logger` feature), and `memory`
//! modules.

#![doc(html_root_url = "https://docs.rs/fluence-sdk-main/0.1.9")]
#![feature(allocator_api)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

extern crate core;

pub mod memory;

#[cfg(feature = "wasm_logger")]
pub mod logger;

#[cfg(feature = "side_module")]
pub mod side_module;

#[cfg(feature = "expect_eth")]
pub mod eth;

#[cfg(feature = "export_allocator")]
pub mod export_allocator;
