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

//! A simple demo application for Fluence.

#![feature(allocator_api)]
#![feature(alloc)]

use fluence;

use log::{error, info, warn};
use std::ptr::NonNull;

/// Initializes `WasmLogger` instance and returns a pointer to error message as a string
/// in memory. Enabled only for a Wasm targets.
#[no_mangle]
#[cfg(target_arch = "wasm32")]
pub unsafe fn init_logger(_: *mut u8, _: usize) -> NonNull<u8> {
    let result = fluence::logger::WasmLogger::init_with_level(log::Level::Info)
        .map(|_| "WasmLogger was successfully initialized".to_string())
        .unwrap_or_else(|err| format!("WasmLogger initialization was failed, cause: {:?}", err));

    warn!("{}\n", result);

    fluence::memory::write_result_to_mem(&result.as_bytes())
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    let user_name = fluence::memory::read_input_from_mem(ptr, len);
    let user_name: String = String::from_utf8(user_name).unwrap();

    info!("{} have been successfully greeted", user_name);

    // return a pointer to the result in memory
    fluence::memory::write_result_to_mem(format!("Hello from Fluence to {}", user_name).as_bytes())
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

fn log_and_panic(msg: String) -> ! {
    error!("{}", msg);
    panic!(msg);
}
