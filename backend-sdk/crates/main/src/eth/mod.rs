/*
 * Copyright 2019 Fluence Labs Limited
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

//! Notifies the VM that this module expects Ethereum blocks.
//!
//! This module contains functions for loading from and storing to memory.

/// A temporary solution to let users configure their ethereum expectations via WASM bytecode:
/// to enable block uploading via invoke method, just export expects_eth method from the module.
#[no_mangle]
pub unsafe fn expects_eth() {}
