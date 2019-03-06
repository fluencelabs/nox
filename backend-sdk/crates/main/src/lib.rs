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

//! The main part of Fluence backend SDK. Contains `export_allocator` (that can be disabled by
//! using `no_export_allocator` feature), `logger` (enabled by `wasm_logger`) and `memory` modules.

#![doc(html_root_url = "https://docs.rs/fluence-sdk-main/0.1.0")]
#![feature(allocator_api)]

extern crate core;

pub mod memory;

// wasm_logger feature should be used only for debugging purposes since the Fluence network doesn't
// support writing messages to a log.
#[cfg(feature = "wasm_logger")]
pub mod logger;

#[cfg(feature = "export_allocator")]
pub mod export_allocator;
