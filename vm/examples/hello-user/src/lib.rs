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

//! Wrapper for counter (a test for Fluence network).
//!
//! Provides the public method `invoke` for increment a counter and get its current state and some
//! public service methods (`allocation`, `deallocation`).

#![feature(allocator_api)]
#![feature(alloc)]
use fluence_sdk as fluence;
use log::{error, info, warn};

use std::num::NonZeroUsize;
use std::ptr::NonNull;

/// Initializes `WasmLogger` instance and returns a pointer to error message as a string
/// in the memory. Enabled only for a Wasm target.
#[no_mangle]
#[cfg(target_arch = "wasm32")]
pub unsafe fn init_logger(_: *mut u8, _: usize) -> NonNull<u8> {
    let result = fluence::logger::WasmLogger::init_with_level(log::Level::Info)
        .map(|_| "WasmLogger was successfully initialized".to_string())
        .unwrap_or_else(|err| format!("WasmLogger initialization was failed, cause: {:?}", err));

    warn!("{}\n", result);

    fluence::memory::write_str_to_mem(&result)
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    let user_name: String = fluence::memory::deref_str(ptr, len);

    info!("{} have been successfully greeted", user_name);

    // return a pointer to the result in memory
    fluence::memory::write_str_to_mem(format!("Hello from Fluence to {}", user_name).as_str())
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation while parameters passing.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Allocation of zero bytes is not allowed."));
    fluence::memory::alloc(non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

/// Deallocates memory area for provided memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Deallocation of zero bytes is not allowed."));
    fluence::memory::dealloc(ptr, non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Deallocate failed for ptr={:?} size={}.", ptr, size));
}

fn log_and_panic(msg: String) -> ! {
    error!("{}", msg);
    panic!(msg);
}
