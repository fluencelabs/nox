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

//! Debugger for Fluence VM

#![feature(allocator_api)]
#![allow(dead_code)]

#[macro_use]
extern crate log;
extern crate fluence_sdk as fluence;

use std::error::Error;
use std::num::NonZeroUsize;
use std::ptr::NonNull;

/// Result for all possible Error types.
type GenResult<T> = Result<T, Box<Error>>;

/// Initializes `WasmLogger` instance and returns a pointer to error message as a string
/// in the memory. Enabled only for a Wasm target.
#[no_mangle]
#[cfg(target_arch = "wasm32")]
pub unsafe fn init_logger(_: *mut u8, _: usize) -> NonNull<u8> {
    let result = fluence::logger::WasmLogger::init_with_level(log::Level::Info)
        .map(|_| "WasmLogger successfully initialized".to_string())
        .unwrap_or_else(|err| format!("WasmLogger initialization failed, cause: {:?}", err));

    let res_ptr = fluence::memory::write_str_to_mem(&result).unwrap_or_else(|_| {
        log_and_panic("Putting result string to the memory failed.".into())
    });

    warn!("{}\n", result);
    res_ptr
}

/// Takes the input, prints it out
#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    info!("invoke starts with ptr={:?}, len={}", ptr, len);
    // memory for the parameter will be deallocated when sql_str was dropped
    let params: String = fluence::memory::deref_str(ptr, len);

    info!("invoke input: {}", params);

    // return pointer to result in memory
    fluence::memory::write_str_to_mem("OK")
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

//
// Public functions for memory management
//

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation while parameters passing.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    info!("allocate starts with size={}", size);
    let non_zero_size = NonZeroUsize::new(size).unwrap_or_else(|| {
        log_and_panic("[Error] Allocation of zero bytes is not allowed.".into())
    });
    fluence::memory::alloc(non_zero_size)
        .unwrap_or_else(|_| log_and_panic(format!("[Error] Allocation of {} bytes failed.", size)))
}

/// Deallocates memory area for provided memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    info!("deallocate starts with ptr={:?}, size={}", ptr, size);
    let non_zero_size = NonZeroUsize::new(size).unwrap_or_else(|| {
        log_and_panic("[Error] Deallocation of zero bytes is not allowed.".into())
    });
    fluence::memory::dealloc(ptr, non_zero_size).unwrap_or_else(|_| {
        log_and_panic(format!(
            "[Error] Deallocate failed for ptr={:?} size={}.",
            ptr, size
        ))
    });
}

fn log_and_panic(msg: String) -> ! {
    error!("{}", msg);
    panic!(msg);
}