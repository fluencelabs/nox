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

//! Allows a module be side module.
//!
//! This module contains functions for loading from and storing to memory.

/// Stores one byte into module memory by the given address.
/// Could be used from other modules for parameter passing.
#[no_mangle]
pub unsafe fn store(address: *mut u8, byte: u8) {
    *address = byte;
}

/// Loads one byte from module memory by the given address.
/// Could be used from other modules for obtaining results.
#[no_mangle]
pub unsafe fn load(address: *mut u8) -> u8 {
    return *address;
}
