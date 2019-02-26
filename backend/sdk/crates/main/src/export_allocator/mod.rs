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

//! This module provides default implementations of [`allocate`] and [`deallocate`] functions that
//! can be used for array passing and returning.
//!
//! To enable it please specify `export_allocator` feature of `fluence` or `fluence-sdk-main`.
//!
//! [`allocate`]: fn.allocate.html
//! [`deallocate`]: fn.deallocate.html

use crate::memory::{alloc, dealloc};
use std::num::NonZeroUsize;
use std::ptr::NonNull;

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation while parameters passing.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Allocation of zero bytes is not allowed."));
    alloc(non_zero_size).unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

/// Deallocates memory area for provided memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Deallocation of zero bytes is not allowed."));
    dealloc(ptr, non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Deallocate failed for ptr={:?} size={}.", ptr, size));
}
