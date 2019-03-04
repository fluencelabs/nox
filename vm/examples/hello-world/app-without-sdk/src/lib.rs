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

use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::num::NonZeroUsize;
use std::ptr::{self, NonNull};

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    let raw_string = Vec::from_raw_parts(ptr, len, len);
    let user_name = String::from_utf8(raw_string).unwrap();

    let result = format!("Hello, world! From user {}", user_name);
    const RESULT_SIZE_BYTES: usize = 4;

    let result_len = result.len();
    let total_len = result_len
        .checked_add(RESULT_SIZE_BYTES)
        .expect("usize overflow occurred");

    // converts array size to bytes in little-endian
    let len_as_bytes: [u8; RESULT_SIZE_BYTES] = mem::transmute((result_len as u32).to_le());

    // allocates a new memory region for the result
    let result_ptr = allocate(total_len);

    // copies length of array to memory
    ptr::copy_nonoverlapping(
        len_as_bytes.as_ptr(),
        result_ptr.as_ptr(),
        RESULT_SIZE_BYTES,
    );

    // copies array to memory
    ptr::copy_nonoverlapping(
        result.as_ptr(),
        result_ptr.as_ptr().add(RESULT_SIZE_BYTES),
        result_len,
    );

    result_ptr
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error]: allocation of zero bytes is not allowed.");
    let layout: Layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())
        .unwrap_or_else(|_| panic!("[Error]: layout creation failed while allocation"));
    Global
        .alloc(layout)
        .unwrap_or_else(|_| panic!("[Error]: allocation of {} bytes failed", size))
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error]: deallocation of zero bytes is not allowed.");
    let layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())
        .unwrap_or_else(|_| panic!("[Error]: layout creation failed while deallocation"));;
    Global.dealloc(ptr, layout);
}
